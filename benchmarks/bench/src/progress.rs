// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::io::{self, IsTerminal, Write};

pub(crate) struct ProgressLine {
    terminal: bool,
    active: bool,
}

impl ProgressLine {
    pub(crate) fn new() -> Self {
        Self {
            terminal: io::stderr().is_terminal(),
            active: false,
        }
    }

    pub(crate) fn update(&mut self, message: &str) {
        if self.terminal {
            eprint!("\r\x1b[2K{message}");
            let _ = io::stderr().flush();
        } else {
            eprintln!("{message}");
        }
        self.active = true;
    }

    pub(crate) fn finish(&mut self) {
        if self.terminal && self.active {
            eprintln!();
        }
        self.active = false;
    }
}

impl Drop for ProgressLine {
    fn drop(&mut self) {
        self.finish();
    }
}

pub(crate) struct BuildProgress {
    line: ProgressLine,
    total: usize,
    built: Vec<String>,
}

impl BuildProgress {
    pub(crate) fn new(total: usize) -> Self {
        Self {
            line: ProgressLine::new(),
            total,
            built: Vec::with_capacity(total),
        }
    }

    pub(crate) fn started(&mut self, package: &str) {
        let name = short_name(package);
        let names = self.built.join(", ");
        let separator = if names.is_empty() { "" } else { ", " };
        self.line.update(&format!(
            "Building ({}/{}): {names}{separator}{name}...",
            self.built.len(),
            self.total
        ));
    }

    pub(crate) fn completed(&mut self, package: &str) {
        self.built.push(short_name(package).to_owned());
        self.line.update(&format!(
            "Building ({}/{}): {}",
            self.built.len(),
            self.total,
            self.built.join(", ")
        ));
    }

    pub(crate) fn finish(&mut self) {
        self.line.finish();
    }
}

fn short_name(package: &str) -> &str {
    package.strip_prefix("quent-bench-rust-").unwrap_or(package)
}
