// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

mod verify;

use clap::ValueEnum;
use quent_instrumentation::FileSystemFormat;
use serde::Serialize;

pub use verify::discarded_events;

pub type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Exporter {
    Noop,
    Ndjson,
    Msgpack,
    Postcard,
}

impl Exporter {
    pub fn file_format(self) -> Option<FileSystemFormat> {
        match self {
            Self::Noop => None,
            Self::Ndjson => Some(FileSystemFormat::Ndjson),
            Self::Msgpack => Some(FileSystemFormat::Msgpack),
            Self::Postcard => Some(FileSystemFormat::Postcard),
        }
    }
}
