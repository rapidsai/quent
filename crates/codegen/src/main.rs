// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod manifest;

#[derive(Parser)]
#[command(version, about = "Validate Quent instrumentation package manifests")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a manifest and its YAML model without writing files.
    Check {
        /// Path to the instrumentation package manifest.
        #[arg(long, default_value = "quent.toml")]
        manifest_path: PathBuf,
    },
}

fn main() -> ExitCode {
    let Cli {
        command: Command::Check { manifest_path },
    } = Cli::parse();
    match manifest::check(&manifest_path) {
        Ok(warnings) => {
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            println!("Valid: {}", manifest_path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
