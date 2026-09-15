// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Simulated NVTX capture for the query-engine simulator.
//!
//! Emits a verbatim [`NvtxEvent`] stream through the same context directory as
//! the model, without linking the Linux-only NVTX C injection layer.

use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
    },
};

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_model::{ContextInner, Observer};
use uuid::Uuid;

/// Domain `0` is NVTX's unnamed default; named libraries occupy `1..`.
const DOMAIN_NAMES: &[&str] = &["CCCL", "libcudf", "CUB", "NCCL", "cuBLAS", "Thrust"];
const CAPTURE_DOMAIN_HANDLES: &[u64] = &[0, 3, 1, 4, 5, 6, 7];

const CATEGORY_NAMES: &[&str] = &["API", "Internal", "Memory", "Compute", "IO"];
const GENERIC_POINTER: i32 = 0x0001_0001;

const LIBCUDF_FRAMES: &[&str] = &[
    "read_parquet",
    "read_chunk_internal",
    "decode_page_data",
    "decode_page_headers",
    "decompress_page_data",
    "decompress",
    "device_decompress",
    "preprocess_levels",
    "preprocess_subpass_pages",
    "build_string_dict_indices",
    "copy_if",
    "finalize_output",
    "binary_operation",
];

const CCCL_KERNELS: &[&str] = &[
    "thrust::copy_if",
    "thrust::transform",
    "cub::DeviceReduce::Reduce",
    "cub::DeviceReduce::TransformReduce",
    "cub::DeviceFor::ForEachN",
    "cub::DeviceTransform::TransformIfStableArgumentAddresses",
];

/// Start from a plausible Linux TID so unnamed threads render as `thread 186143`.
const OS_THREAD_BASE: u32 = 186_143;

pub const DEFAULT_MAX_NVTX_RANGES: usize = 20_000;

/// How many NVTX domains, categories, marks, and extra nested ranges to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvtxLayout {
    /// Distinct domains to declare. `0` disables NVTX emission.
    pub num_domains: usize,
    /// Named categories declared in every domain. `0` leaves ranges uncategorized.
    pub num_categories: usize,
    /// Instant marks emitted per query.
    pub num_marks: usize,
    /// Extra nested libcudf frames inside each scan.
    pub num_nested_ranges: usize,
    /// Emit per-task NVTX ranges on 1 in N operator tasks (`0` skips them).
    pub task_every: usize,
    /// Maximum ranges emitted across the entire simulation.
    pub max_ranges: usize,
}

impl Default for NvtxLayout {
    fn default() -> Self {
        Self {
            num_domains: 3,
            num_categories: CATEGORY_NAMES.len(),
            num_marks: 2,
            num_nested_ranges: 0,
            task_every: 1,
            max_ranges: DEFAULT_MAX_NVTX_RANGES,
        }
    }
}

/// Workload categories shared across every simulated domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvtxCategory {
    Api,
    Internal,
    Memory,
    Compute,
    Io,
}

impl NvtxCategory {
    const fn index(self) -> usize {
        match self {
            Self::Api => 0,
            Self::Internal => 1,
            Self::Memory => 2,
            Self::Compute => 3,
            Self::Io => 4,
        }
    }
}

/// Side-stream NVTX emitter sharing a simulator context id.
pub struct NvtxCapture {
    context_id: Uuid,
    observer: Observer<NvtxEventEntity>,
    layout: NvtxLayout,
    next_thread_id: AtomicU32,
    next_range_id: AtomicU64,
    next_resource_handle: AtomicU64,
    next_string_handle: AtomicU64,
    ranges_emitted: AtomicUsize,
    registered_strings: Mutex<HashMap<(u64, String), u64>>,
}

impl NvtxCapture {
    /// Discard every event. Used when the simulator exporter is no-op.
    pub fn noop(context_id: Uuid, layout: NvtxLayout) -> Self {
        Self {
            context_id,
            observer: Observer::noop(),
            layout,
            next_thread_id: AtomicU32::new(OS_THREAD_BASE),
            next_range_id: AtomicU64::new(1),
            next_resource_handle: AtomicU64::new(1),
            next_string_handle: AtomicU64::new(1),
            ranges_emitted: AtomicUsize::new(0),
            registered_strings: Mutex::new(HashMap::new()),
        }
    }

    /// Attach an NVTX stream to `context_id` using `provider`.
    pub fn try_new(
        context_id: Uuid,
        provider: &impl quent_model::io::ExporterProvider<NvtxEventEntity>,
        layout: NvtxLayout,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let inner = ContextInner::try_new(context_id)?;
        let observer =
            inner.block_on(async { inner.observer::<NvtxEventEntity>(provider).await })?;
        Ok(Self {
            context_id,
            observer,
            layout,
            next_thread_id: AtomicU32::new(OS_THREAD_BASE),
            next_range_id: AtomicU64::new(1),
            next_resource_handle: AtomicU64::new(1),
            next_string_handle: AtomicU64::new(1),
            ranges_emitted: AtomicUsize::new(0),
            registered_strings: Mutex::new(HashMap::new()),
        })
    }

    pub fn layout(&self) -> NvtxLayout {
        self.layout
    }

    /// Select task offsets uniformly across one thread's workload.
    pub fn sampled_task_offsets(&self, task_count: usize, max_samples: usize) -> Vec<usize> {
        let every = self.layout.task_every;
        if every == 0 || task_count == 0 || max_samples == 0 {
            return Vec::new();
        }
        let candidate_count = task_count.div_ceil(every);
        let sample_count = max_samples.min(candidate_count);
        if sample_count == 1 {
            return vec![(candidate_count / 2) * every];
        }
        (0..sample_count)
            .map(|index| index * (candidate_count - 1) / (sample_count - 1) * every)
            .collect()
    }

    #[cfg(test)]
    fn ranges_emitted(&self) -> usize {
        self.ranges_emitted.load(Ordering::Relaxed)
    }

    /// Domain handle `0..num_domains`, wrapping `index`. `0` is the default domain.
    pub fn domain_at(&self, index: usize) -> u64 {
        if self.layout.num_domains != 0 {
            domain_handle(index % self.layout.num_domains)
        } else {
            0
        }
    }

    /// Domain `index` when it was declared; `None` if the layout is smaller.
    pub fn try_domain(&self, index: usize) -> Option<u64> {
        (index < self.layout.num_domains).then(|| domain_handle(index))
    }

    /// Category id `1..=num_categories`, wrapping `index`. `0` when none were declared.
    pub fn category_at(&self, index: usize) -> u32 {
        match self.layout.num_categories {
            0 => 0,
            n => 1 + (index % n) as u32,
        }
    }

    pub fn category_id(&self, category: NvtxCategory) -> u32 {
        self.category_at(category.index())
    }

    pub fn cccl_kernel_name(index: usize) -> &'static str {
        CCCL_KERNELS[index % CCCL_KERNELS.len()]
    }

    /// Domain and category names matching [`Self::domain_at`] / [`Self::category_at`].
    pub fn declare_schema(&self) {
        for index in 0..self.layout.num_domains {
            let domain = domain_handle(index);
            if domain != 0 {
                self.emit(NvtxEvent::DomainCreate {
                    domain,
                    name: domain_name(index),
                });
            }
            for category in 1..=self.layout.num_categories as u32 {
                self.emit(NvtxEvent::NameCategory {
                    domain,
                    category,
                    name: category_name(category),
                });
            }
        }
    }

    /// Destroy every named domain after its simulated entities have closed.
    pub fn destroy_schema(&self) {
        for index in (0..self.layout.num_domains).rev() {
            let domain = domain_handle(index);
            if domain != 0 {
                self.emit(NvtxEvent::DomainDestroy { domain });
            }
        }
    }

    /// Allocate a raw OS thread id without naming it (renders as `thread {id}`).
    pub fn alloc_thread(&self) -> u32 {
        self.next_thread_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate a stable simulated OS thread id and name it.
    pub fn name_thread(&self, name: &str) -> u32 {
        let thread_id = self.alloc_thread();
        self.emit(NvtxEvent::NameThread {
            thread_id,
            name: name.to_owned(),
        });
        thread_id
    }

    pub fn mark(&self, domain: u64, message: &str, category: u32) {
        self.emit(NvtxEvent::Mark {
            domain,
            attributes: self.attributes(domain, message, category),
        });
    }

    /// Push a nested per-thread range; popping happens when the guard drops.
    pub fn push(
        &self,
        domain: u64,
        thread_id: u32,
        message: &str,
        category: u32,
    ) -> NvtxPushGuard<'_> {
        self.push_if(true, domain, thread_id, message, category)
    }

    pub fn push_if(
        &self,
        enabled: bool,
        domain: u64,
        thread_id: u32,
        message: &str,
        category: u32,
    ) -> NvtxPushGuard<'_> {
        let active = enabled && self.try_reserve_range();
        if active {
            self.emit(NvtxEvent::RangePush {
                domain,
                thread_id,
                attributes: self.attributes(domain, message, category),
            });
        }
        NvtxPushGuard {
            capture: self,
            domain,
            thread_id,
            active,
        }
    }

    /// Extra libcudf frames that pop innermost-first when the stack drops.
    pub fn push_nested(&self, thread_id: u32, count: usize) -> NvtxPushStack<'_> {
        let Some(domain) = self.try_domain(2) else {
            return NvtxPushStack { guards: Vec::new() };
        };
        NvtxPushStack {
            guards: (0..count)
                .map(|index| {
                    let name = LIBCUDF_FRAMES[index % LIBCUDF_FRAMES.len()];
                    self.push(domain, thread_id, name, self.category_at(index))
                })
                .collect(),
        }
    }

    /// Open a process-wide range; the matching end is emitted when the guard drops.
    pub fn start(&self, domain: u64, message: &str, category: u32) -> NvtxStartGuard<'_> {
        let range_id = self.try_reserve_range().then(|| {
            let range_id = self.next_range_id.fetch_add(1, Ordering::Relaxed);
            self.emit(NvtxEvent::RangeStart {
                domain,
                range_id,
                attributes: self.attributes(domain, message, category),
            });
            range_id
        });
        NvtxStartGuard {
            capture: self,
            domain,
            range_id,
        }
    }

    /// Create a generic-pointer resource; its matching destroy is emitted on drop.
    pub fn resource(&self, domain: u64, message: &str) -> NvtxResourceGuard<'_> {
        let handle = self.next_resource_handle.fetch_add(1, Ordering::Relaxed);
        self.emit(NvtxEvent::ResourceCreate {
            domain,
            handle,
            identifier_type: GENERIC_POINTER,
            identifier: handle,
            message: Some(self.message(domain, message)),
        });
        NvtxResourceGuard {
            capture: self,
            handle,
        }
    }

    fn try_reserve_range(&self) -> bool {
        if self.layout.num_domains == 0 {
            return false;
        }
        self.ranges_emitted
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < self.layout.max_ranges).then_some(count + 1)
            })
            .is_ok()
    }

    fn emit(&self, event: NvtxEvent) {
        if self.layout.num_domains == 0 {
            return;
        }
        self.observer.emit(self.context_id, event);
    }

    fn attributes(&self, domain: u64, message: &str, category: u32) -> NvtxEventAttributes {
        NvtxEventAttributes {
            category,
            color: None,
            message: Some(self.message(domain, message)),
            payload: None,
        }
    }

    fn message(&self, domain: u64, message: &str) -> NvtxMessage {
        if domain == 0 {
            return NvtxMessage::String(message.to_owned());
        }
        let key = (domain, message.to_owned());
        let handle = {
            let mut strings = self.registered_strings.lock().unwrap();
            *strings.entry(key).or_insert_with(|| {
                let handle = self.next_string_handle.fetch_add(1, Ordering::Relaxed);
                self.emit(NvtxEvent::RegisterString {
                    domain,
                    handle,
                    string: message.to_owned(),
                });
                handle
            })
        };
        NvtxMessage::RegisteredHandle(handle)
    }
}

/// Pops the matching [`NvtxEvent::RangePush`] on drop.
pub struct NvtxPushGuard<'a> {
    capture: &'a NvtxCapture,
    domain: u64,
    thread_id: u32,
    active: bool,
}

impl Drop for NvtxPushGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            self.capture.emit(NvtxEvent::RangePop {
                domain: self.domain,
                thread_id: self.thread_id,
            });
        }
    }
}

/// Nested push guards that drop innermost first, matching NVTX's per-thread stack.
pub struct NvtxPushStack<'a> {
    guards: Vec<NvtxPushGuard<'a>>,
}

impl Drop for NvtxPushStack<'_> {
    fn drop(&mut self) {
        while self.guards.pop().is_some() {}
    }
}

/// Ends the matching [`NvtxEvent::RangeStart`] on drop.
pub struct NvtxStartGuard<'a> {
    capture: &'a NvtxCapture,
    domain: u64,
    range_id: Option<u64>,
}

impl Drop for NvtxStartGuard<'_> {
    fn drop(&mut self) {
        if let Some(range_id) = self.range_id {
            self.capture.emit(NvtxEvent::RangeEnd {
                domain: self.domain,
                range_id,
            });
        }
    }
}

/// Destroys the matching [`NvtxEvent::ResourceCreate`] on drop.
pub struct NvtxResourceGuard<'a> {
    capture: &'a NvtxCapture,
    handle: u64,
}

impl Drop for NvtxResourceGuard<'_> {
    fn drop(&mut self) {
        self.capture.emit(NvtxEvent::ResourceDestroy {
            handle: self.handle,
        });
    }
}

fn domain_name(index: usize) -> String {
    DOMAIN_NAMES
        .get(index.saturating_sub(1))
        .copied()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("domain-{}", domain_handle(index)))
}

fn domain_handle(index: usize) -> u64 {
    CAPTURE_DOMAIN_HANDLES
        .get(index)
        .copied()
        .unwrap_or(index as u64 + 1)
}

fn category_name(category: u32) -> String {
    CATEGORY_NAMES
        .get(category.saturating_sub(1) as usize)
        .copied()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("category-{category}"))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    use nvtx_analyzer::NvtxModelBuilder;
    use quent_model::EventCallback;

    use super::*;

    fn collect(
        layout: NvtxLayout,
        body: impl FnOnce(&NvtxCapture),
    ) -> Vec<quent_model::Event<NvtxEventEntity>> {
        let events = Arc::new(Mutex::new(Vec::<quent_model::Event<NvtxEventEntity>>::new()));
        let exporter = EventCallback::new({
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        });
        let context_id = Uuid::now_v7();
        {
            let nvtx = NvtxCapture::try_new(context_id, &exporter, layout).unwrap();
            body(&nvtx);
        }
        std::mem::take(&mut *events.lock().unwrap())
    }

    #[test]
    fn nested_push_pop_reconstructs_two_spans() {
        let captured = collect(NvtxLayout::default(), |nvtx| {
            nvtx.declare_schema();
            let thread_id = nvtx.alloc_thread();
            let _outer = nvtx.push(nvtx.domain_at(0), thread_id, "task", nvtx.category_at(1));
            let _inner = nvtx.push(nvtx.domain_at(1), thread_id, "alloc", nvtx.category_at(0));
        });
        let model = NvtxModelBuilder::build(captured);
        assert_eq!(model.spans().len(), 2);
        assert!(model.anomalies().is_faithful());
    }

    #[test]
    fn emits_every_core_nvtx_event_type() {
        let captured = collect(NvtxLayout::default(), |nvtx| {
            nvtx.declare_schema();
            let thread_id = nvtx.name_thread("main");
            nvtx.mark(
                nvtx.domain_at(0),
                "checkpoint",
                nvtx.category_id(NvtxCategory::Api),
            );
            {
                let _push = nvtx.push(
                    nvtx.domain_at(1),
                    thread_id,
                    "push/pop",
                    nvtx.category_id(NvtxCategory::Internal),
                );
                let _start = nvtx.start(
                    nvtx.domain_at(0),
                    "start/end",
                    nvtx.category_id(NvtxCategory::Compute),
                );
                let _resource = nvtx.resource(nvtx.domain_at(2), "device buffer");
            }
            nvtx.destroy_schema();
        });
        let kinds: HashSet<_> = captured
            .iter()
            .map(|event| match event.data.0 {
                NvtxEvent::RangePush { .. } => "RangePush",
                NvtxEvent::RangePop { .. } => "RangePop",
                NvtxEvent::RangeStart { .. } => "RangeStart",
                NvtxEvent::RangeEnd { .. } => "RangeEnd",
                NvtxEvent::Mark { .. } => "Mark",
                NvtxEvent::DomainCreate { .. } => "DomainCreate",
                NvtxEvent::DomainDestroy { .. } => "DomainDestroy",
                NvtxEvent::RegisterString { .. } => "RegisterString",
                NvtxEvent::NameCategory { .. } => "NameCategory",
                NvtxEvent::NameThread { .. } => "NameThread",
                NvtxEvent::ResourceCreate { .. } => "ResourceCreate",
                NvtxEvent::ResourceDestroy { .. } => "ResourceDestroy",
            })
            .collect();
        assert_eq!(
            kinds,
            HashSet::from([
                "RangePush",
                "RangePop",
                "RangeStart",
                "RangeEnd",
                "Mark",
                "DomainCreate",
                "DomainDestroy",
                "RegisterString",
                "NameCategory",
                "NameThread",
                "ResourceCreate",
                "ResourceDestroy",
            ])
        );

        let model = NvtxModelBuilder::build(captured);
        assert_eq!(model.marks().len(), 1);
        assert_eq!(model.resources().count(), 1);
        assert!(model.anomalies().is_faithful());
    }

    #[test]
    fn workload_categories_resolve_to_distinct_ids() {
        let nvtx = NvtxCapture::noop(Uuid::now_v7(), NvtxLayout::default());
        assert_eq!(nvtx.category_id(NvtxCategory::Api), 1);
        assert_eq!(nvtx.category_id(NvtxCategory::Internal), 2);
        assert_eq!(nvtx.category_id(NvtxCategory::Memory), 3);
        assert_eq!(nvtx.category_id(NvtxCategory::Compute), 4);
        assert_eq!(nvtx.category_id(NvtxCategory::Io), 5);
    }

    #[test]
    fn default_range_limit_is_twenty_thousand() {
        assert_eq!(NvtxLayout::default().max_ranges, 20_000);
    }

    #[test]
    fn schema_declares_named_domains_skipping_default() {
        let layout = NvtxLayout {
            num_domains: 3,
            num_categories: 2,
            num_marks: 0,
            num_nested_ranges: 0,
            task_every: 1,
            max_ranges: DEFAULT_MAX_NVTX_RANGES,
        };
        let captured = collect(layout, |nvtx| nvtx.declare_schema());
        let domains: Vec<_> = captured
            .iter()
            .filter_map(|event| match &event.data.0 {
                NvtxEvent::DomainCreate { domain, name } => Some((*domain, name.as_str())),
                _ => None,
            })
            .collect();
        let categories = captured
            .iter()
            .filter(|event| matches!(event.data.0, NvtxEvent::NameCategory { .. }))
            .count();
        assert_eq!(domains, vec![(3, "CCCL"), (1, "libcudf")]);
        assert_eq!(categories, 6);
    }

    #[test]
    fn named_domains_use_registered_strings_but_default_domain_does_not() {
        let captured = collect(NvtxLayout::default(), |nvtx| {
            let thread_id = nvtx.alloc_thread();
            let _default = nvtx.push(0, thread_id, "pipeline", 0);
            let _libcudf = nvtx.push(nvtx.domain_at(2), thread_id, "read_parquet", 0);
        });
        assert!(captured.iter().any(|event| matches!(
            &event.data.0,
            NvtxEvent::RangePush {
                domain: 0,
                attributes: NvtxEventAttributes {
                    message: Some(NvtxMessage::String(message)),
                    ..
                },
                ..
            } if message == "pipeline"
        )));
        assert!(captured.iter().any(|event| matches!(
            &event.data.0,
            NvtxEvent::RegisterString { domain: 1, string, .. } if string == "read_parquet"
        )));
        assert!(captured.iter().any(|event| matches!(
            event.data.0,
            NvtxEvent::RangePush {
                domain: 1,
                attributes: NvtxEventAttributes {
                    message: Some(NvtxMessage::RegisteredHandle(_)),
                    ..
                },
                ..
            }
        )));
    }

    #[test]
    fn zero_domains_emits_nothing() {
        let captured = collect(
            NvtxLayout {
                num_domains: 0,
                num_categories: 4,
                num_marks: 8,
                num_nested_ranges: 8,
                task_every: 1,
                max_ranges: DEFAULT_MAX_NVTX_RANGES,
            },
            |nvtx| {
                nvtx.declare_schema();
                nvtx.mark(1, "nope", 1);
            },
        );
        assert!(captured.is_empty());
    }

    #[test]
    fn task_samples_honor_stride_and_span_the_workload() {
        let sampled = NvtxCapture::noop(
            Uuid::now_v7(),
            NvtxLayout {
                task_every: 5,
                ..NvtxLayout::default()
            },
        );
        assert_eq!(sampled.sampled_task_offsets(100, 4), vec![0, 30, 60, 95]);
        let none = NvtxCapture::noop(
            Uuid::now_v7(),
            NvtxLayout {
                task_every: 0,
                ..NvtxLayout::default()
            },
        );
        assert!(none.sampled_task_offsets(100, 4).is_empty());
    }

    #[test]
    fn task_samples_include_both_ends_when_possible() {
        let nvtx = NvtxCapture::noop(Uuid::now_v7(), NvtxLayout::default());
        assert_eq!(nvtx.sampled_task_offsets(100, 4), vec![0, 33, 66, 99]);
        assert_eq!(nvtx.sampled_task_offsets(100, 1), vec![50]);
    }

    #[test]
    fn range_limit_is_global_across_threads_and_range_types() {
        const MAX_RANGES: usize = 20;
        let captured = collect(
            NvtxLayout {
                max_ranges: MAX_RANGES,
                ..NvtxLayout::default()
            },
            |nvtx| {
                std::thread::scope(|scope| {
                    for thread_id in 1u32..=4 {
                        scope.spawn(move || {
                            for index in 0usize..10 {
                                if index.is_multiple_of(2) {
                                    let _range = nvtx.push(0, thread_id, "push", 0);
                                } else {
                                    let _range = nvtx.start(0, "start", 0);
                                }
                            }
                        });
                    }
                });
                assert_eq!(nvtx.ranges_emitted(), MAX_RANGES);
            },
        );
        let model = NvtxModelBuilder::build(captured);
        assert_eq!(model.spans().len(), MAX_RANGES);
        assert!(model.anomalies().is_faithful());
    }

    #[test]
    fn zero_range_limit_still_emits_marks() {
        let captured = collect(
            NvtxLayout {
                max_ranges: 0,
                ..NvtxLayout::default()
            },
            |nvtx| {
                let thread_id = nvtx.alloc_thread();
                let _push = nvtx.push(0, thread_id, "skipped", 0);
                let _start = nvtx.start(0, "skipped", 0);
                nvtx.mark(0, "kept", 0);
                assert_eq!(nvtx.ranges_emitted(), 0);
            },
        );
        let model = NvtxModelBuilder::build(captured);
        assert!(model.spans().is_empty());
        assert_eq!(model.marks().len(), 1);
        assert!(model.anomalies().is_faithful());
    }
}
