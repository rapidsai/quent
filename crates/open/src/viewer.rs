// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generate, build, and serve the viewer crate for a [`ViewerSpec`], then open a
//! browser at the served URL.

use std::net::TcpListener as StdTcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::process::Command;

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;
use crate::wrapper::{self, PORT_ENV, ROOT_ENV, WRAPPER_PACKAGE};

/// Generate, build, and serve the viewer for `spec`. Blocks serving until the
/// viewer process exits (e.g. Ctrl-C). Opens a browser unless `no_browser`.
pub async fn open(spec: &ViewerSpec, no_browser: bool, print_url: bool) -> Result<()> {
    let crate_dir = build_dir(spec)?;
    wrapper::generate(spec, &crate_dir)?;
    let bin = cargo_build(&crate_dir).await?;
    let output_root = stage_output_root(&crate_dir, &spec.root)?;
    let result = serve(&output_root, &bin, no_browser, print_url).await;
    // Best-effort cleanup of this run's staged root (the cached build is kept).
    let _ = std::fs::remove_dir_all(&output_root);
    result
}

/// Stage a clean output root containing only the requested `context`, symlinked
/// under its own UUID name. The server scans an output root of `<context-uuid>/`
/// directories; isolating to one context serves exactly what was asked and avoids
/// tripping over sibling contexts that may use a different format.
///
/// The root is unique per process so concurrent runs sharing a cached build dir
/// do not clobber each other's staged root.
fn stage_output_root(crate_dir: &Path, context: &Path) -> Result<PathBuf> {
    let context = context.canonicalize()?;
    let name = context.file_name().ok_or_else(|| {
        OpenError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "context path has no final component",
        ))
    })?;
    let root = crate_dir.join(format!("serve-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    symlink_dir(&context, &root.join(name))?;
    Ok(root)
}

/// Symlink the context directory into the staged output root.
#[cfg(unix)]
fn symlink_dir(src: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink_dir(_src: &Path, _link: &Path) -> Result<()> {
    Err(OpenError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "serving local artifacts requires symlink support",
    )))
}

/// Cache directory for this viewer's generated crate and build, under the user
/// cache dir keyed by [`ViewerSpec::cache_key`] so identical specs are reused.
fn build_dir(spec: &ViewerSpec) -> Result<PathBuf> {
    let base = dirs::cache_dir().ok_or(OpenError::NoCacheDir)?;
    Ok(base
        .join("quent")
        .join("open")
        .join("builds")
        .join(spec.cache_key()))
}

/// Run `cargo build --release` in `crate_dir` (output inherited so the user sees
/// progress), returning the built binary path.
///
/// The first build fetches the pinned git sources and compiles the embedded UI,
/// which invokes `pnpm`/`node`; both must be on `PATH`. Subsequent builds reuse
/// the cached `crate_dir`.
async fn cargo_build(crate_dir: &Path) -> Result<PathBuf> {
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(crate_dir)
        .status()
        .await
        .map_err(|source| OpenError::Spawn {
            what: "cargo build".into(),
            source,
        })?;
    if !status.success() {
        return Err(OpenError::Build {
            status: status.to_string(),
        });
    }
    Ok(crate_dir
        .join("target")
        .join("release")
        .join(WRAPPER_PACKAGE))
}

/// Spawn the built viewer serving `output_root`, print/open its URL, and run
/// until it exits.
async fn serve(output_root: &Path, bin: &Path, no_browser: bool, print_url: bool) -> Result<()> {
    let port = free_port()?;
    let url = format!("http://127.0.0.1:{port}/");

    let mut child = Command::new(bin)
        .env(ROOT_ENV, output_root)
        .env(PORT_ENV, port.to_string())
        .spawn()
        .map_err(|source| OpenError::Spawn {
            what: "viewer".into(),
            source,
        })?;

    if print_url {
        println!("{url}");
    }
    if !no_browser {
        // Wait for the server to accept connections before opening the browser.
        wait_until_ready(port).await;
        if let Err(e) = open::that(&url) {
            eprintln!("could not open a browser ({e}); open {url} manually");
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(OpenError::ViewerExited {
            status: status.to_string(),
        });
    }
    Ok(())
}

/// Pick a currently-free localhost TCP port. There is a small race between this
/// and the viewer binding it, acceptable for a local dev tool.
fn free_port() -> Result<u16> {
    let listener = StdTcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

/// Poll `port` until it accepts a connection (server up) or a few seconds pass.
async fn wait_until_ready(port: u16) {
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
