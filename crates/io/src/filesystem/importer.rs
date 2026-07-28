// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use quent_io_types::{Importer, ImporterProvider, ImporterResult};

use crate::filesystem::Format;

/// Options for importing events from the filesystem in the given `format`.
/// `path` is either a directory containing the event file (located by the
/// format's extension) or a direct file path.
#[derive(Debug, Clone)]
pub struct Options {
    pub format: Format,
    pub path: PathBuf,
}

impl<T> ImporterProvider<T> for Options
where
    T: 'static,
    for<'a> T: serde::Deserialize<'a>,
{
    fn create_importer(&self) -> ImporterResult<Box<dyn Importer<T>>> {
        match self.format {
            #[cfg(feature = "ndjson")]
            Format::Ndjson => Ok(Box::new(quent_io_ndjson::NdjsonImporter::try_new(
                &quent_io_ndjson::NdjsonImporterOptions {
                    path: self.path.clone(),
                },
            )?) as Box<dyn Importer<T>>),
            #[cfg(feature = "msgpack")]
            Format::Msgpack => Ok(Box::new(quent_io_msgpack::MsgpackImporter::try_new(
                &quent_io_msgpack::MsgpackImporterOptions {
                    path: self.path.clone(),
                },
            )?) as Box<dyn Importer<T>>),
            #[cfg(feature = "postcard")]
            Format::Postcard => Ok(Box::new(quent_io_postcard::PostcardImporter::try_new(
                &quent_io_postcard::PostcardImporterOptions {
                    path: self.path.clone(),
                },
            )?) as Box<dyn Importer<T>>),
        }
    }
}
