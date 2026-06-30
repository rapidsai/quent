// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build and serve viewers for discovered contexts. Contexts sharing a build
//! spec (same analyzer + pinned commits + format) share one viewer. Builds run
//! one at a time — they compile the full quent stack, including a `pnpm` UI
//! build in the shared git checkout, so concurrent `cargo` invocations would
//! race — then the built viewers are served in parallel.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use backon::{ConstantBuilder, Retryable};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinSet;

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;
use crate::wrapper::{self, ADDR_ENV, ROOT_ENV, WRAPPER_PACKAGE};

/// Viewer to build: representative [`ViewerSpec`] plus all contexts sharing it.
pub struct ViewerGroup {
    pub spec: ViewerSpec,
    pub contexts: Vec<PathBuf>,
}

/// A built viewer ready to serve: its binary, cache dir, and the contexts it covers.
struct BuiltViewer {
    bin: PathBuf,
    crate_dir: PathBuf,
    contexts: Vec<PathBuf>,
    label: String,
}

/// Build each group's viewer (serially, see module docs), then serve the built
/// viewers in parallel — each bound on `host`, announcing its URL when ready.
/// Open a browser only for a single viewer. Block until all viewers exit (e.g.
/// Ctrl-C); a failed build or viewer does not stop the others.
pub async fn open_all(groups: Vec<ViewerGroup>, no_browser: bool, host: IpAddr) -> Result<()> {
    let total: usize = groups.iter().map(|g| g.contexts.len()).sum();
    println!(
        "discovered {total} context(s) -> {} viewer(s)",
        groups.len()
    );
    let open_browser = !no_browser && groups.len() == 1;

    let mut failures = 0usize;
    let mut built = Vec::new();
    for group in groups {
        match build_one(group).await {
            Ok(viewer) => built.push(viewer),
            Err(e) => {
                failures += 1;
                eprintln!("viewer build failed: {e}");
            }
        }
    }

    let mut set = JoinSet::new();
    for viewer in built {
        set.spawn(async move { serve_one(viewer, open_browser, host).await });
    }
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                failures += 1;
                eprintln!("viewer failed: {e}");
            }
            Err(e) => {
                failures += 1;
                eprintln!("viewer task error: {e}");
            }
        }
    }
    if failures > 0 {
        Err(OpenError::ViewersFailed { count: failures })
    } else {
        Ok(())
    }
}

/// Generate and build one group's viewer crate.
async fn build_one(group: ViewerGroup) -> Result<BuiltViewer> {
    let ViewerGroup { spec, contexts } = group;
    let label = format!("{} — {} context(s)", spec.describe(), contexts.len());
    println!("building: {label}");

    let crate_dir = build_dir(&spec)?;
    wrapper::generate(&spec, &crate_dir)?;
    let bin = cargo_build(&crate_dir).await?;
    Ok(BuiltViewer {
        bin,
        crate_dir,
        contexts,
        label,
    })
}

/// Serve one built viewer over all its contexts.
async fn serve_one(viewer: BuiltViewer, open_browser: bool, host: IpAddr) -> Result<()> {
    let BuiltViewer {
        bin,
        crate_dir,
        contexts,
        label,
    } = viewer;
    let output_root = stage_output_root(&crate_dir, &contexts)?;
    let result = serve(&output_root, &bin, &label, open_browser, host).await;
    // Best-effort cleanup of this run's staged root; keep the cached build.
    let _ = std::fs::remove_dir_all(&output_root);
    result
}

/// Cache dir for this viewer's generated crate/build, keyed by
/// [`ViewerSpec::cache_key`] under the user cache dir so identical specs are reused.
fn build_dir(spec: &ViewerSpec) -> Result<PathBuf> {
    let base = dirs::cache_dir().ok_or(OpenError::NoCacheDir)?;
    Ok(base
        .join("quent")
        .join("open")
        .join("builds")
        .join(spec.cache_key()))
}

/// Run `cargo build --release` in `crate_dir` and return the built binary path,
/// read from Cargo's JSON output so a custom target dir/triple is handled. Build
/// diagnostics go to `<crate_dir>/build.log`; on failure the log's tail is folded
/// into the error. The target dir is pinned under `crate_dir` so a user's global
/// `CARGO_TARGET_DIR`/`build.target-dir` can't collide one viewer's binary with
/// another's.
///
/// The first build fetches the pinned git sources and compiles the embedded UI,
/// which invokes `pnpm`/`node`; both must be on `PATH`. Subsequent builds reuse
/// the cached `crate_dir`.
async fn cargo_build(crate_dir: &Path) -> Result<PathBuf> {
    let log_path = crate_dir.join("build.log");
    let log = std::fs::File::create(&log_path)?;
    let mut child = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", crate_dir.join("target"))
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|source| OpenError::Spawn {
            what: "cargo build".into(),
            source,
        })?;

    let mut json = Vec::new();
    child
        .stdout
        .take()
        .expect("piped stdout")
        .read_to_end(&mut json)
        .await?;
    let status = child.wait().await?;
    if !status.success() {
        // Compiler diagnostics are rendered into the JSON stream; cargo's own
        // output (build scripts, the final error summary) is on stderr in the
        // log. Include both in full so the cause is never truncated away.
        let mut detail = rendered_diagnostics(&json);
        detail.push_str(&std::fs::read_to_string(&log_path).unwrap_or_default());
        return Err(OpenError::Build {
            status: format!("{status}\n{detail}"),
        });
    }
    wrapper_executable(&json).ok_or_else(|| OpenError::Build {
        status: format!("cargo build reported no `{WRAPPER_PACKAGE}` executable"),
    })
}

/// Find the wrapper binary's path in cargo's `--message-format=json`
/// `compiler-artifact` messages (avoids assuming a target-dir layout).
fn wrapper_executable(stdout: &[u8]) -> Option<PathBuf> {
    std::str::from_utf8(stdout).ok()?.lines().find_map(|line| {
        let msg: serde_json::Value = serde_json::from_str(line).ok()?;
        let is_wrapper =
            msg["reason"] == "compiler-artifact" && msg["target"]["name"] == WRAPPER_PACKAGE;
        is_wrapper
            .then(|| msg["executable"].as_str().map(PathBuf::from))
            .flatten()
    })
}

/// Last 20 lines of a build log, for surfacing why a build failed.
/// Concatenate the human-rendered compiler diagnostics from cargo's JSON message
/// stream (`--message-format=json-render-diagnostics` carries them here rather
/// than on stderr).
fn rendered_diagnostics(stdout: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(stdout) else {
        return String::new();
    };
    text.lines()
        .filter_map(|line| {
            let msg: serde_json::Value = serde_json::from_str(line).ok()?;
            msg["message"]["rendered"].as_str().map(str::to_owned)
        })
        .collect()
}

/// Stage a clean output root, symlinking each `context` under its UUID name.
/// The server scans `<context-uuid>/` directories; isolating requested contexts
/// serves exactly them and avoids unrelated siblings that may use another format.
/// The root is per process so concurrent runs sharing a cached build do not
/// clobber each other.
fn stage_output_root(crate_dir: &Path, contexts: &[PathBuf]) -> Result<PathBuf> {
    let root = crate_dir.join(format!("serve-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let mut linked = std::collections::HashSet::new();
    for context in contexts {
        let context = context.canonicalize()?;
        let name = context.file_name().ok_or_else(|| {
            OpenError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "context path has no final component",
            ))
        })?;
        // A context id (the dir name) is unique within one export, so this only
        // fires when the same context is reached via two distinct paths — a copied
        // context, or overlapping `paths` args that each contain it. Those dedup to
        // distinct canonical paths but one link name; keep the first and skip the
        // rest rather than aborting the whole viewer on the colliding link.
        if !linked.insert(name.to_owned()) {
            eprintln!(
                "warning: ignoring duplicate context id `{}` at {}",
                name.to_string_lossy(),
                context.display()
            );
            continue;
        }
        symlink_dir(&context, &root.join(name))?;
    }
    Ok(root)
}

/// Symlink a context directory into the staged output root.
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

/// Spawn the viewer for `output_root` bound on `host`, announce its URL once it
/// accepts connections, and run until exit.
async fn serve(
    output_root: &Path,
    bin: &Path,
    label: &str,
    open_browser: bool,
    host: IpAddr,
) -> Result<()> {
    let addr = free_port(host)?;
    // An unspecified host (`0.0.0.0`/`::`) is not browseable; show and probe the
    // matching loopback instead (the server may be bound v6-only on `::`).
    let reachable = match addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => (Ipv4Addr::LOCALHOST, addr.port()).into(),
        IpAddr::V6(ip) if ip.is_unspecified() => (Ipv6Addr::LOCALHOST, addr.port()).into(),
        _ => addr,
    };
    let url = format!("http://{reachable}/");

    let mut child = Command::new(bin)
        .env(ROOT_ENV, output_root)
        .env(ADDR_ENV, addr.to_string())
        .spawn()
        .map_err(|source| OpenError::Spawn {
            what: "viewer".into(),
            source,
        })?;

    if wait_until_ready(reachable).await {
        println!("ready: {label}  {url}");
        if open_browser && let Err(e) = open::that(&url) {
            eprintln!("could not open a browser ({e}); open {url} manually");
        }
    } else {
        eprintln!("warning: {label} did not start listening at {url} within the timeout");
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(OpenError::ViewerExited {
            status: status.to_string(),
        });
    }
    Ok(())
}

/// Reserve a free TCP port on `host`, returning the full bind address; the small
/// race before the viewer binds it is acceptable for a local dev tool.
fn free_port(host: IpAddr) -> Result<SocketAddr> {
    let listener = StdTcpListener::bind((host, 0))?;
    Ok(listener.local_addr()?)
}

/// Poll `addr` until it accepts a connection, returning `false` on timeout.
async fn wait_until_ready(addr: SocketAddr) -> bool {
    (|| async { tokio::net::TcpStream::connect(addr).await })
        .retry(
            ConstantBuilder::default()
                .with_delay(Duration::from_millis(100))
                .with_max_times(50),
        )
        .await
        .is_ok()
}
