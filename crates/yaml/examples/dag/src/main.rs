// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused)]
mod instrumentation {
    include!(concat!(env!("OUT_DIR"), "/query_plan.rs"));
}

use instrumentation::{Context, Noop, Operator, Plan, PlanEdge, QueryPlan};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::<QueryPlan>::try_new(Noop)?;

    let mut plan = context.observer::<Plan>().handle();
    plan.created()?;

    let mut source = context.observer::<Operator>().handle();
    source.declared(plan.as_entity_ref())?;

    let mut target = context.observer::<Operator>().handle();
    target.declared(plan.as_entity_ref())?;

    let mut edge = context.observer::<PlanEdge>().handle();
    edge.connected(
        plan.as_entity_ref(),
        source.as_entity_ref(),
        target.as_entity_ref(),
    )?;

    Ok(())
}
