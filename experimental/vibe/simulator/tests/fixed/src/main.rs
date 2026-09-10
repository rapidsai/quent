// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binary wrapper around the fixed query-engine event emitter. See the crate
//! library for the scenario.

use clap::Parser;
use quent_io::clap::ExporterArgs;
use quent_simulator_fixed::emit;
use quent_simulator_instrumentation as instr;

type SimulatorContext = instr::Context<instr::Simulator>;

#[derive(Parser, Debug)]
#[command(name = "quent-simulator-fixed")]
#[command(about = "Emits a fixed query-engine telemetry stream", long_about = None)]
struct Args {
    #[command(flatten)]
    exporter: ExporterArgs,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ctx = match args.exporter.into_options() {
        Some(provider) => SimulatorContext::try_new(provider)?,
        None => SimulatorContext::try_new(instr::Noop)?,
    };
    emit(&ctx);
    Ok(())
}
