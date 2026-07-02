// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binary wrapper around the fixed query-engine event emitter. See the crate
//! library for the scenario.

use clap::Parser;
use quent_exporter::clap::ExporterArgs;
use quent_query_engine_fixed::emit;
use quent_simulator_instrumentation::SimulatorContext;

#[derive(Parser, Debug)]
#[command(name = "quent-query-engine-fixed")]
#[command(about = "Emits a fixed query-engine telemetry stream", long_about = None)]
struct Args {
    #[command(flatten)]
    exporter: ExporterArgs,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ctx = SimulatorContext::try_new(args.exporter.into_options())?;
    emit(&ctx);
    Ok(())
}
