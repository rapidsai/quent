// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A [`Loader`] that fetches a benchmark run's telemetry from the Benchmarking
//! API and materializes it as local context directories.
//!
//! Note: the "Benchmarking API" is an internal RAPIDS/NVIDIA service, so this
//! mode is not useful to non-NVIDIA users for now — hence it lives behind the
//! off-by-default `db` feature.
//!
//! Telemetry packaging is not fixed across engines, so this loader does not
//! assume a layout: it downloads the run's archive assets into a scratch dir,
//! extracts them, and then reuses [`discover_contexts`] to find every `model.qmi`
//! in the unpacked tree — i.e. it "greps for the telemetry" rather than hard-coding
//! a shape. The fetched contexts flow through the same trust/build/serve pipeline
//! as local ones.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use quent_build_info::SIDECAR_FILE_NAME;
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Loader;
use crate::archive::{MAX_ENTRIES, MAX_RUN_UNPACKED_BYTES, extract_archive, is_archive};
use crate::error::{OpenError, Result};
use crate::spec::discover_contexts;

/// Run-wide cap on compressed bytes downloaded across all assets, so a run with
/// many archives can't fill the disk despite each asset looking small.
const MAX_RUN_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// Cap on a `model.qmi` sidecar, which `open()` reads whole into memory (via
/// `ArtifactInfo::read_sidecar`) before the trust gate; keeps a remote archive
/// from OOMing the process with a giant sidecar under the archive limit.
const MAX_SIDECAR_BYTES: u64 = 4 * 1024 * 1024;

/// Stall guards on API/download requests (not a cap on total download time).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// A [`Loader`] backed by the Benchmarking API database.
///
/// Owns a scratch [`TempDir`](tempfile::TempDir) that holds the extracted
/// telemetry; [`run`](crate::run) keeps the loader alive for the whole call, so the
/// scratch dir outlives serving.
pub struct DbLoader {
    base_url: String,
    token: String,
    run: String,
    scratch: tempfile::TempDir,
}

impl DbLoader {
    /// Create a loader for `run` (an integer run id or its `run_id` UUID) against
    /// the API at `base_url`, authenticating with the bearer `token`.
    pub fn new(base_url: String, token: String, run: String) -> Result<Self> {
        Ok(Self {
            base_url,
            token,
            run,
            scratch: tempfile::tempdir()?,
        })
    }

    /// Join an API path onto the base URL, tolerating a trailing/leading slash.
    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// GET `url` with bearer auth and query params, decoding a JSON body; a
    /// non-success status becomes [`OpenError::Api`] (with the body for context).
    async fn get_json<T: DeserializeOwned>(
        &self,
        client: &Client,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let response = client
            .get(url)
            .bearer_auth(&self.token)
            .query(query)
            .send()
            .await?;
        Ok(check(response).await?.json::<T>().await?)
    }

    /// Resolve the requested run to its integer primary key. An integer is used
    /// directly; otherwise the value is treated as a `run_id` UUID and matched by
    /// paging the run list (the API has no uuid filter, so this is a linear scan).
    async fn resolve_run(&self, client: &Client) -> Result<i64> {
        if let Ok(id) = self.run.parse::<i64>() {
            return Ok(id);
        }
        let url = self.endpoint("api/benchmark-runs/");
        let mut offset: i64 = 0;
        loop {
            let page: Paged<BenchmarkRunSummary> = self
                .get_json(
                    client,
                    &url,
                    &[("limit", "100"), ("offset", &offset.to_string())],
                )
                .await?;
            // `run_id` is a UUID; compare case-insensitively so a differently-cased
            // spelling still matches.
            if let Some(found) = page
                .items
                .iter()
                .find(|r| r.run_id.eq_ignore_ascii_case(&self.run))
            {
                return Ok(found.id);
            }
            offset += page.items.len() as i64;
            if page.items.is_empty() || offset >= page.count {
                return Err(OpenError::RunNotFound {
                    run: self.run.clone(),
                });
            }
        }
    }

    /// List every asset attached to `run_id`, following pagination.
    async fn list_assets(&self, client: &Client, run_id: i64) -> Result<Vec<AssetSummary>> {
        let url = self.endpoint("api/assets/");
        let mut assets = Vec::new();
        let mut offset: i64 = 0;
        loop {
            let page: Paged<AssetSummary> = self
                .get_json(
                    client,
                    &url,
                    &[
                        ("benchmark_run_id", &run_id.to_string()),
                        ("limit", "100"),
                        ("offset", &offset.to_string()),
                    ],
                )
                .await?;
            let count = page.count;
            offset += page.items.len() as i64;
            let done = page.items.is_empty() || offset >= count;
            assets.extend(page.items);
            if done {
                break;
            }
        }
        Ok(assets)
    }

    /// Resolve an asset's presigned download URL (authenticated), then stream its
    /// bytes to `dest_file` directly from storage *without* the API token (the URL
    /// is signed). Streaming (rather than buffering) bounds memory; the download is
    /// capped, and errors have their URL stripped since the presigned URL carries
    /// credentials in its query string.
    async fn download_asset(
        &self,
        client: &Client,
        asset_id: i64,
        dest_file: &Path,
        run_remaining: &mut u64,
    ) -> Result<()> {
        let download: AssetDownload = self
            .get_json(
                client,
                &self.endpoint(&format!("api/assets/{asset_id}/download/")),
                &[("redirect", "false")],
            )
            .await?;
        let mut response = client
            .get(&download.download_url)
            .send()
            .await
            .map_err(strip_url)?;
        if !response.status().is_success() {
            return Err(OpenError::Api {
                status: response.status().to_string(),
                body: response.text().await.map_err(strip_url)?,
            });
        }
        let mut file = std::fs::File::create(dest_file)?;
        while let Some(chunk) = response.chunk().await.map_err(strip_url)? {
            let len = chunk.len() as u64;
            if len > *run_remaining {
                return Err(OpenError::BadArtifactLayout {
                    detail: format!(
                        "run downloads exceed the {MAX_RUN_DOWNLOAD_BYTES}-byte budget"
                    ),
                });
            }
            *run_remaining -= len;
            file.write_all(&chunk)?;
        }
        Ok(())
    }
}

/// Strip the URL from a reqwest error so a presigned (credential-bearing) URL never
/// reaches logs or CLI output.
fn strip_url(error: reqwest::Error) -> OpenError {
    OpenError::Http(error.without_url())
}

impl Loader for DbLoader {
    async fn load(&self) -> Result<Vec<PathBuf>> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()?;
        let run_id = self.resolve_run(&client).await?;

        // Run-wide budgets, drawn down across every asset (not reset per asset).
        let mut download_remaining = MAX_RUN_DOWNLOAD_BYTES;
        let mut unpacked_remaining = MAX_RUN_UNPACKED_BYTES;
        let mut entries_remaining = MAX_ENTRIES;

        let mut extracted = 0usize;
        for asset in self.list_assets(&client, run_id).await? {
            if !is_archive(&asset.original_filename) {
                eprintln!(
                    "skipping non-telemetry asset {} ({})",
                    asset.original_filename, asset.id
                );
                continue;
            }
            // Stage each asset in its own subdir so archives that share a context
            // uuid can't overwrite each other before discovery.
            let dest = self.scratch.path().join(asset.id.to_string());
            std::fs::create_dir_all(&dest)?;
            let archive = self.scratch.path().join(format!("{}.download", asset.id));
            self.download_asset(&client, asset.id, &archive, &mut download_remaining)
                .await?;
            extract_archive(
                &dest,
                &asset.original_filename,
                &archive,
                &mut unpacked_remaining,
                &mut entries_remaining,
            )?;
            std::fs::remove_file(&archive).ok(); // reclaim the compressed copy
            extracted += 1;
        }
        if extracted == 0 {
            return Err(OpenError::NoTelemetryAssets {
                run: self.run.clone(),
            });
        }

        // Find the telemetry wherever it landed in the unpacked tree, and reject
        // layouts the viewer can't serve (see [`validate_context_names`]).
        let contexts = discover_contexts(&[self.scratch.path().to_path_buf()])?;
        validate_context_names(&contexts)?;
        // Guard the sidecar `open()` will read whole into memory before trusting it.
        for context in &contexts {
            let sidecar = context.join(SIDECAR_FILE_NAME);
            let size = std::fs::metadata(&sidecar)?.len();
            if size > MAX_SIDECAR_BYTES {
                return Err(OpenError::BadArtifactLayout {
                    detail: format!(
                        "{} is {size} bytes, over the {MAX_SIDECAR_BYTES}-byte sidecar limit",
                        sidecar.display()
                    ),
                });
            }
        }
        Ok(contexts)
    }
}

/// The viewer indexes contexts by their `<uuid>` directory name and stages them by
/// name, so every discovered context must be named by a UUID (not, e.g., sidecar
/// files at an archive root) and no two may share a name (else `stage_output_root`
/// would silently drop one). Reject both rather than serve partial telemetry.
fn validate_context_names(contexts: &[PathBuf]) -> Result<()> {
    let mut seen = HashSet::new();
    for context in contexts {
        let name = context.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !is_uuid(name) {
            return Err(OpenError::BadArtifactLayout {
                detail: format!(
                    "context directory `{name}` is not named by a UUID; \
                     archives must contain `<context-uuid>/` directories"
                ),
            });
        }
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(OpenError::BadArtifactLayout {
                detail: format!("context uuid `{name}` appears in more than one asset"),
            });
        }
    }
    Ok(())
}

/// Whether `name` is a UUID in the canonical lowercase hyphenated form. The viewer
/// addresses each context by `Uuid::to_string()` (lowercase hyphenated), so a
/// non-canonical spelling (uppercase, or the hyphen-less simple form) would list
/// but fail to import on a case-sensitive filesystem; require the exact form the
/// importer will look up.
fn is_uuid(name: &str) -> bool {
    uuid::Uuid::try_parse(name).is_ok_and(|uuid| uuid.hyphenated().to_string() == name)
}

/// Turn a non-success HTTP response into [`OpenError::Api`], preserving the body.
async fn check(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().to_string();
    let body = response.text().await.unwrap_or_default();
    Err(OpenError::Api { status, body })
}

/// A paginated API list response (`{ items, count }`).
#[derive(Deserialize)]
struct Paged<T> {
    items: Vec<T>,
    count: i64,
}

/// The fields of a benchmark run we need to resolve a UUID to its primary key.
#[derive(Deserialize)]
struct BenchmarkRunSummary {
    id: i64,
    run_id: String,
}

/// The fields of an asset we need to decide whether/how to fetch it.
#[derive(Deserialize)]
struct AssetSummary {
    id: i64,
    original_filename: String,
}

/// The presigned download URL for an asset.
#[derive(Deserialize)]
struct AssetDownload {
    download_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uuid_requires_canonical_form() {
        assert!(is_uuid("0198e5a1-2b3c-7d4e-8f90-1a2b3c4d5e6f")); // canonical lowercase hyphenated
        assert!(!is_uuid("0198E5A1-2B3C-7D4E-8F90-1A2B3C4D5E6F")); // uppercase: importer uses lowercase
        assert!(!is_uuid("0198e5a12b3c7d4e8f901a2b3c4d5e6f")); // simple form isn't what's addressed
        assert!(!is_uuid("ctx-uuid"));
        assert!(!is_uuid("not-a-uuid"));
    }

    #[test]
    fn validate_context_names_requires_uuid_and_rejects_duplicates() {
        let uuid = "0198e5a1-2b3c-7d4e-8f90-1a2b3c4d5e6f";
        assert!(validate_context_names(&[PathBuf::from(format!("/s/{uuid}"))]).is_ok());
        // A root-level asset dir (named by the integer asset id) has no UUID.
        assert!(validate_context_names(&[PathBuf::from("/s/42")]).is_err());
        // The same context uuid across two assets would be silently dropped downstream.
        assert!(
            validate_context_names(&[
                PathBuf::from(format!("/a/{uuid}")),
                PathBuf::from(format!("/b/{uuid}")),
            ])
            .is_err()
        );
    }
}
