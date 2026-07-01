// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use quent_build_info::SIDECAR_FILE_NAME;
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Loader;
use crate::error::{OpenError, Result};
use crate::spec::discover_contexts;

/// Asset filename suffixes treated as (possibly compressed) tar archives of
/// telemetry. Other assets on a run (parquet traces, nsys reports, …) are skipped.
const ARCHIVE_SUFFIXES: &[&str] = &[".tar", ".tar.gz", ".tgz", ".tar.zst", ".tzst"];

/// Run-wide cap on compressed bytes downloaded across all assets, so a run with
/// many archives can't fill the disk despite each asset looking small.
const MAX_RUN_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// Run-wide cap on bytes extracted across all assets (their staged dirs all live
/// until the viewer exits), so aggregate extraction can't exhaust the disk.
const MAX_RUN_UNPACKED_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// Per-archive cap on extracted bytes. Bounds a single archive — and so the
/// in-memory read of any one GNU long-name/PAX extension record, which `tar`
/// buffers before we can inspect it — independently of the run-wide budget.
///
/// Residual: `tar` exposes no hook to cap an extension record at the KB scale a
/// legit path needs, so a hostile long-name is bounded only to this per-archive
/// cap, not smaller. Acceptable given the internal, trusted source; a raw
/// tar-header parser would be needed to tighten it further.
const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Run-wide cap on the number of extracted entries, so an archive of millions of
/// tiny files/dirs can't exhaust inodes under the byte budgets.
const MAX_ENTRIES: u64 = 1_000_000;

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

/// Extract a downloaded tar archive (optionally gzip/zstd compressed) at `src` into
/// `dest`. Entries that escape `dest` or aren't plain files/dirs (symlinks,
/// hardlinks, devices) are rejected; extracted bytes are capped per-archive and
/// against the run-wide `unpacked_remaining`, and the entry count against
/// `entries_remaining`.
fn extract_archive(
    dest: &Path,
    filename: &str,
    src: &Path,
    unpacked_remaining: &mut u64,
    entries_remaining: &mut u64,
) -> Result<()> {
    let reader = std::io::BufReader::new(std::fs::File::open(src)?);
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        unpack_tar(
            zstd::stream::read::Decoder::new(reader)?,
            dest,
            unpacked_remaining,
            entries_remaining,
        )
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        unpack_tar(
            flate2::read::GzDecoder::new(reader),
            dest,
            unpacked_remaining,
            entries_remaining,
        )
    } else {
        unpack_tar(reader, dest, unpacked_remaining, entries_remaining)
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

/// Whether an asset filename looks like a (possibly compressed) tar archive.
fn is_archive(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    ARCHIVE_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// A reader that errors once its per-archive quota or the shared run-wide budget
/// is exhausted, so an oversized stream (e.g. a decompression bomb, or a giant GNU
/// long-name/PAX record `tar` buffers internally) fails loudly instead of being
/// silently truncated at a [`Read::take`] EOF that `tar` would treat as success.
struct LimitReader<'a, R> {
    inner: R,
    archive_remaining: u64,
    run_remaining: &'a mut u64,
}

impl<R: Read> Read for LimitReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let allowed = self.archive_remaining.min(*self.run_remaining);
        if allowed == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "archive exceeds the unpack size limit",
            ));
        }
        let cap = buf.len().min(allowed as usize);
        let read = self.inner.read(&mut buf[..cap])?;
        self.archive_remaining -= read as u64;
        *self.run_remaining -= read as u64;
        Ok(read)
    }
}

/// Unpack a tar archive under `root`. Rejects entries that escape `root` (paths
/// with `..` or an absolute/root prefix — [`tar`] skips those, returning `false`)
/// and entries that aren't plain files or directories (symlinks/hardlinks/devices,
/// which could redirect writes outside `root`). Extracted bytes are capped
/// per-archive and against the shared `unpacked_remaining`; the entry count is
/// capped against `entries_remaining`.
fn unpack_tar<R: Read>(
    reader: R,
    root: &Path,
    unpacked_remaining: &mut u64,
    entries_remaining: &mut u64,
) -> Result<()> {
    let reader = LimitReader {
        inner: reader,
        archive_remaining: MAX_ARCHIVE_UNPACKED_BYTES,
        run_remaining: unpacked_remaining,
    };
    let mut archive = tar::Archive::new(reader);
    // Don't apply archive file/dir modes: these are throwaway scratch dirs, and a
    // read-only dir header (e.g. `0555`) would otherwise block extracting its
    // children or removing the tree on cleanup.
    archive.set_preserve_permissions(false);
    archive.set_preserve_mtime(false);
    for entry in archive.entries()? {
        if *entries_remaining == 0 {
            return Err(OpenError::BadArtifactLayout {
                detail: format!("run exceeds the {MAX_ENTRIES}-entry archive limit"),
            });
        }
        *entries_remaining -= 1;
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            let path = entry_path(&entry);
            return Err(OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("refusing non-regular archive entry ({kind:?}): {path}"),
            )));
        }
        if !entry.unpack_in(root)? {
            return Err(OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "refusing archive entry that escapes the scratch dir: {}",
                    entry_path(&entry)
                ),
            )));
        }
    }
    Ok(())
}

/// An archive entry's path as a lossy string, for error messages.
fn entry_path<R: Read>(entry: &tar::Entry<'_, R>) -> String {
    entry
        .path()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
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

    fn add_entry(builder: &mut tar::Builder<Vec<u8>>, path: &str, data: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, path, data).unwrap();
    }

    /// Build an in-memory tar of a `<uuid>/model.qmi` + `<entity>/x.ndjson` context.
    fn context_tar() -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        add_entry(&mut builder, "ctx-uuid/model.qmi", b"{}");
        add_entry(&mut builder, "ctx-uuid/engine/x.ndjson", b"");
        builder.into_inner().unwrap()
    }

    /// Write archive bytes to a temp file (`extract_archive` reads from a path).
    fn archive_file(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    /// Generous run budgets for extraction tests that don't exercise the caps.
    fn budgets() -> (u64, u64) {
        (MAX_RUN_UNPACKED_BYTES, MAX_ENTRIES)
    }

    #[test]
    fn is_archive_matches_tar_family_only() {
        assert!(is_archive("run.tar"));
        assert!(is_archive("run.tar.zst"));
        assert!(is_archive("RUN.TAR.GZ"));
        assert!(!is_archive("traces.parquet"));
        assert!(!is_archive("report.nsys-rep"));
    }

    #[test]
    fn extract_tar_then_discover_finds_context() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&context_tar());
        let (mut bytes, mut entries) = budgets();
        extract_archive(
            dest.path(),
            "run.tar",
            archive.path(),
            &mut bytes,
            &mut entries,
        )
        .unwrap();

        let found = discover_contexts(&[dest.path().to_path_buf()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "ctx-uuid");
        assert!(found[0].join(SIDECAR_FILE_NAME).is_file());
    }

    #[test]
    fn extract_tolerates_readonly_dir_headers() {
        // A read-only dir header (0555) before its child must not block extracting
        // the child (we don't preserve archive modes on scratch dirs).
        let mut builder = tar::Builder::new(Vec::new());
        let mut dir = tar::Header::new_gnu();
        dir.set_entry_type(tar::EntryType::Directory);
        dir.set_size(0);
        dir.set_mode(0o555);
        dir.set_cksum();
        builder
            .append_data(&mut dir, "ro/", std::io::empty())
            .unwrap();
        add_entry(&mut builder, "ro/child.txt", b"hi");
        let archive = archive_file(&builder.into_inner().unwrap());

        let dest = tempfile::tempdir().unwrap();
        let (mut bytes, mut entries) = budgets();
        extract_archive(
            dest.path(),
            "run.tar",
            archive.path(),
            &mut bytes,
            &mut entries,
        )
        .unwrap();
        assert!(dest.path().join("ro/child.txt").is_file());
    }

    #[test]
    fn extract_rejects_too_many_entries() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&context_tar()); // two entries
        let (mut bytes, mut entries) = (MAX_RUN_UNPACKED_BYTES, 1);
        assert!(
            extract_archive(
                dest.path(),
                "run.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

    /// Hand-build a one-entry ustar archive with an arbitrary `name`/`typeflag`
    /// (the safe `tar::Builder` API rejects `..` and normalizes links, so we can't
    /// use it to forge malicious entries).
    fn raw_tar(name: &str, link: &str, typeflag: u8, data: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000644\0"); // mode
        header[108..116].copy_from_slice(b"0000000\0"); // uid
        header[116..124].copy_from_slice(b"0000000\0"); // gid
        header[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes()); // size
        header[136..148].copy_from_slice(b"00000000000\0"); // mtime
        header[156] = typeflag;
        header[157..157 + link.len()].copy_from_slice(link.as_bytes()); // linkname
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        "); // checksum field spaces before summing
        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());

        let mut out = header.to_vec();
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0); // pad final block
        out.extend(std::iter::repeat_n(0, 1024)); // two zero blocks terminate the archive
        out
    }

    #[test]
    fn extract_rejects_path_traversal() {
        let dest = tempfile::tempdir().unwrap();
        let archive = archive_file(&raw_tar("../escape.txt", "", b'0', b"pwned"));
        let (mut bytes, mut entries) = budgets();
        assert!(
            extract_archive(
                dest.path(),
                "evil.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

    #[test]
    fn extract_rejects_symlink_entry() {
        let dest = tempfile::tempdir().unwrap();
        // A symlink (typeflag '2') pointing outside could redirect later writes.
        let archive = archive_file(&raw_tar("link", "/etc/passwd", b'2', b""));
        let (mut bytes, mut entries) = budgets();
        assert!(
            extract_archive(
                dest.path(),
                "evil.tar",
                archive.path(),
                &mut bytes,
                &mut entries
            )
            .is_err()
        );
    }

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

    #[test]
    fn limit_reader_errors_when_run_budget_exhausted() {
        let data = [0u8; 100];
        let mut run = 50u64;
        let mut reader = LimitReader {
            inner: &data[..],
            archive_remaining: 1_000,
            run_remaining: &mut run,
        };
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
    }

    #[test]
    fn limit_reader_errors_when_archive_cap_exhausted() {
        let data = [0u8; 100];
        let mut run = 1_000u64;
        let mut reader = LimitReader {
            inner: &data[..],
            archive_remaining: 50,
            run_remaining: &mut run,
        };
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
    }
}
