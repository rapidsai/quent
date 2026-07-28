// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-memory test helpers for the `Simulator` model.

use quent_io_callback::RecordedEvent;
use quent_model::Event;

use crate::{
    NetworkEvent, SimulatorEvent, ThreadPoolEvent, channel, engine, memory, operator, plan, port,
    processor, query, query_group, task, worker,
};

/// Reconstruct the `Simulator` event stream from events captured in memory by a
/// callback exporter, e.g. for feeding an analyzer in tests.
///
/// Events whose entity is not part of the model are skipped.
pub fn events_from_recorded(
    recorded: impl IntoIterator<Item = RecordedEvent>,
) -> Vec<Event<SimulatorEvent>> {
    // Try to downcast the type-erased event to each concrete `Event<T>`; the
    // matching one lifts into the umbrella `SimulatorEvent`. `downcast` hands the
    // box back on a miss, so attempts thread through it.
    macro_rules! rebuild {
        ($($ty:ty),+ $(,)?) => {
            |rec: RecordedEvent| {
                let mut any = rec.event;
                $(
                    any = match any.downcast::<Event<$ty>>() {
                        Ok(event) => {
                            return Some(Event::new(
                                event.id,
                                event.timestamp,
                                SimulatorEvent::from(event.data),
                            ));
                        }
                        Err(any) => any,
                    };
                )+
                let _ = any;
                None
            }
        };
    }
    recorded
        .into_iter()
        .filter_map(rebuild!(
            engine::EngineEvent,
            worker::WorkerEvent,
            query_group::QueryGroupEvent,
            query::QueryEvent,
            plan::PlanEvent,
            operator::OperatorEvent,
            port::PortEvent,
            task::TaskEvent,
            ThreadPoolEvent,
            NetworkEvent,
            memory::MemoryEvent,
            processor::ProcessorEvent,
            channel::ChannelEvent,
        ))
        .collect()
}
