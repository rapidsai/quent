// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused)]
mod instrumentation {
    include!(concat!(env!("OUT_DIR"), "/logging_sink.rs"));
}

use instrumentation::{AppLog, Context, LoggingSink, Noop};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::<LoggingSink>::try_new(Noop)?;
    let log = context.observer::<AppLog>().handle();

    log.info(
        "application started".to_owned(),
        Some("startup".to_owned()),
        Some(file!().to_owned()),
        Some(line!()),
        Some(module_path!().to_owned()),
        Some("main".to_owned()),
    )?;
    log.warning(
        "retrying request".to_owned(),
        Some("network".to_owned()),
        Some(file!().to_owned()),
        Some(line!()),
        Some(module_path!().to_owned()),
        Some("main".to_owned()),
        Some("transient".to_owned()),
    )?;

    Ok(())
}
