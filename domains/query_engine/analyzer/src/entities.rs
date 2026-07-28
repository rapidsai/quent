// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic entity-list query over any application FSM type.

use std::collections::HashSet;

use quent_analyzer::{
    AnalyzerResult,
    fsm::{FsmUsages, collection::FsmCollection},
    resource::Usage,
};
use quent_time::{TimeNanoSec, TimeUnixNanoSec, span::SpanUnixNanoSec, to_nanosecs};
use quent_ui::{
    FiniteStateMachine,
    entities::{
        request::{EntityListFilter, EntitySortKey, Sort, SortDir},
        response::EntityListResponse,
    },
    paginate::PageParams,
};
use uuid::Uuid;

/// A single entity-list query with its scope already resolved to resource IDs.
pub struct ListQuery<'a> {
    pub scope: Option<&'a HashSet<Uuid>>,
    pub window: SpanUnixNanoSec,
    pub filter: &'a EntityListFilter,
    pub sort: Sort,
    pub page: Option<PageParams>,
    pub epoch: TimeUnixNanoSec,
}

/// List the FSMs matching the scope, window, and filters, ranked and paged.
///
/// `keep` is an extra application predicate for filters outside the generic
/// contract, e.g. by operator.
pub fn list_entities<M, P>(
    model: &M,
    keep: P,
    query: ListQuery<'_>,
) -> AnalyzerResult<EntityListResponse>
where
    M: FsmCollection,
    M::Fsm: for<'a> FsmUsages<'a>,
    P: Fn(&M::Fsm) -> bool,
{
    let ranked = model
        .fsms()
        .filter(|f| keep(f))
        .filter_map(|f| entry_matches(f, query.scope, query.window, query.filter).map(|m| (f, m)))
        .collect();
    finalize(ranked, query.sort, query.page, query.epoch)
}

/// The ranking metric if the FSM passes the filters, else `None`.
fn entry_matches<'a, F>(
    fsm: &'a F,
    scope: Option<&HashSet<Uuid>>,
    window: SpanUnixNanoSec,
    filter: &EntityListFilter,
) -> Option<TimeNanoSec>
where
    F: FsmUsages<'a>,
{
    if filter
        .entity_type_name
        .as_deref()
        .is_some_and(|name| fsm.type_name() != name)
    {
        return None;
    }
    let metric = usage_metric(fsm, scope, window)?;
    let min_usage = filter.min_usage_s.map(to_nanosecs);
    min_usage.is_none_or(|t| metric >= t).then_some(metric)
}

/// Sort the scored candidates, slice the page, and convert to UI FSMs.
fn finalize<'a, F>(
    mut ranked: Vec<(&'a F, TimeNanoSec)>,
    sort: Sort,
    page: Option<PageParams>,
    epoch: TimeUnixNanoSec,
) -> AnalyzerResult<EntityListResponse>
where
    F: FsmUsages<'a>,
{
    ranked.sort_by(|(fa, ma), (fb, mb)| {
        let by_key = match sort.key {
            EntitySortKey::UsageDuration => ma.cmp(mb),
        };
        let by_key = match sort.dir {
            SortDir::Asc => by_key,
            SortDir::Desc => by_key.reverse(),
        };
        by_key.then_with(|| fa.id().cmp(&fb.id()))
    });

    let total = ranked.len() as u32;

    let page_iter: Box<dyn Iterator<Item = (&F, TimeNanoSec)>> = match page {
        Some(p) => Box::new(
            ranked
                .into_iter()
                // Saturate so an out-of-range page skips everything (empty page)
                // instead of overflowing usize on a 32-bit target.
                .skip(p.page.saturating_mul(p.max) as usize)
                .take(p.max as usize),
        ),
        None => Box::new(ranked.into_iter()),
    };

    let items = page_iter
        .map(|(f, _)| FiniteStateMachine::try_from_fsm(f, epoch))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EntityListResponse { items, total })
}

/// The longest single usage span within the window on a scope resource, or any
/// resource when `scope` is `None`.
fn usage_metric<'a, F>(
    fsm: &'a F,
    scope: Option<&HashSet<Uuid>>,
    window: SpanUnixNanoSec,
) -> Option<TimeNanoSec>
where
    F: FsmUsages<'a>,
{
    let longest = fsm
        .usages_with_state_names()
        .filter(|(_, u)| scope.is_none_or(|s| s.contains(&u.resource_id())))
        .filter_map(|(_, u)| u.span().intersection(&window))
        .map(|s| s.duration())
        .max();

    match scope {
        Some(_) => longest,
        None => Some(longest.unwrap_or(0)),
    }
}
