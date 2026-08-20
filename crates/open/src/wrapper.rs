// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generate a standalone wrapper crate serving the quent UI for a model via
//! its analyzer. Dependencies are git-pinned to sidecar commits (clone, don't
//! patch), so analyzer and quent versions line up. For an in-repo model, both
//! share one git source+rev, which Cargo deduplicates to one checkout.

use std::collections::BTreeMap;
use std::path::Path;

use cargo_manifest::{
    Dependency, DependencyDetail, Edition, Manifest, MaybeInherited, Package, Publish, Workspace,
};
use quote::{format_ident, quote};

use crate::error::Result;
use crate::spec::ViewerSpec;

/// Name of the generated wrapper package (also the built binary name).
pub const WRAPPER_PACKAGE: &str = "quent-open-viewer";

/// Cargo package of quent's I/O crate (export/import formats).
pub const IO_PACKAGE: &str = "quent-io";
/// Cargo package of the I/O crate before its rename to [`IO_PACKAGE`], for
/// artifacts pinned to quent revisions that predate the rename.
pub const LEGACY_IO_PACKAGE: &str = "quent-exporter";
/// Cargo package that provides the optional NVTX HTTP routes.
pub const NVTX_SERVER_PACKAGE: &str = "nvtx-server";

/// Whether a generated wrapper targets a quent revision with NVTX routes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NvtxRoutes {
    Enabled,
    Disabled,
}

impl NvtxRoutes {
    fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}

/// Wrapper env var for the output root: a directory of `<context-uuid>/`
/// context directories.
pub const ROOT_ENV: &str = "QUENT_OPEN_ROOT";
/// Env var the wrapper reads for the `ip:port` socket address to bind.
pub const ADDR_ENV: &str = "QUENT_OPEN_ADDR";

/// Write the wrapper crate (`Cargo.toml` + `src/main.rs`) into `crate_dir`.
/// `io_package` is the name of quent's I/O crate at the pinned revision
/// ([`IO_PACKAGE`], or [`LEGACY_IO_PACKAGE`] for revisions predating the rename).
pub fn generate(
    spec: &ViewerSpec,
    crate_dir: &Path,
    io_package: &str,
    nvtx_routes: NvtxRoutes,
) -> Result<()> {
    std::fs::create_dir_all(crate_dir.join("src"))?;
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        cargo_toml(spec, io_package, nvtx_routes),
    )?;
    std::fs::write(crate_dir.join("src/main.rs"), main_rs(spec, nvtx_routes))?;
    Ok(())
}

/// A git-pinned dependency, with optional features.
fn git_dep(url: String, rev: &str, features: &[&str]) -> Dependency {
    Dependency::Detailed(DependencyDetail {
        git: Some(url),
        rev: Some(rev.to_string()),
        features: (!features.is_empty()).then(|| features.iter().map(|f| f.to_string()).collect()),
        ..Default::default()
    })
}

/// Wrapper `Cargo.toml`, built with `cargo-manifest`: pin quent crates to
/// `quent.{remote,commit}` and the analyzer to `analyzer.{remote,commit}`; the
/// empty `[workspace]` keeps the generated crate out of any parent workspace.
fn cargo_toml(spec: &ViewerSpec, io_package: &str, nvtx_routes: NvtxRoutes) -> String {
    let quent = spec.quent.cargo_url();
    let q_rev = spec.quent.commit.as_str();
    let nvtx_dependency = nvtx_routes.is_enabled().then(|| {
        (
            NVTX_SERVER_PACKAGE.to_string(),
            git_dep(quent.clone(), q_rev, &[]),
        )
    });
    let dependencies: BTreeMap<String, Dependency> = BTreeMap::from([
        (
            "quent-query-engine-server".to_string(),
            git_dep(quent.clone(), q_rev, &["ui"]),
        ),
        (
            "quent-query-engine-analyzer".to_string(),
            git_dep(quent.clone(), q_rev, &[]),
        ),
        (
            // All formats enabled so the analyzer can detect the artifact's format at runtime.
            io_package.to_string(),
            git_dep(quent, q_rev, &["ndjson", "msgpack", "postcard"]),
        ),
        (
            spec.analyzer_package.clone(),
            git_dep(spec.analyzer.cargo_url(), &spec.analyzer.commit, &[]),
        ),
        ("axum".to_string(), Dependency::Simple("0.8".to_string())),
        (
            "tokio".to_string(),
            Dependency::Detailed(DependencyDetail {
                version: Some("1".to_string()),
                features: Some(
                    ["macros", "net", "rt-multi-thread"]
                        .iter()
                        .map(|f| f.to_string())
                        .collect(),
                ),
                ..Default::default()
            }),
        ),
        ("uuid".to_string(), Dependency::Simple("1".to_string())),
    ])
    .into_iter()
    .chain(nvtx_dependency)
    .collect();

    let mut package = Package::new(WRAPPER_PACKAGE.to_string(), "0.0.0".to_string());
    package.edition = Some(MaybeInherited::Local(Edition::E2024));
    package.publish = Some(MaybeInherited::Local(Publish::Flag(false)));

    // `()` metadata so `Workspace::default()` applies (the default `Value`
    // metadata type is not `Default`); we emit no `[package.metadata]` anyway.
    let manifest = Manifest::<(), ()> {
        package: Some(package),
        workspace: Some(Workspace::default()),
        dependencies: Some(dependencies),
        ..Default::default()
    };

    format!(
        "# Generated by quent-open. Do not edit.\n{}",
        toml::to_string(&manifest).expect("wrapper manifest serializes")
    )
}

/// Wrapper `src/main.rs`: wire `<analyzer>::Viewer`'s analyzer/importer into
/// `analyzer_service_router` and serve it. Root (`<context-uuid>/` subdirs) and
/// bind address come from env so one built binary serves any artifacts.
fn main_rs(spec: &ViewerSpec, nvtx_routes: NvtxRoutes) -> String {
    let analyzer_crate = format_ident!("{}", spec.analyzer_crate());
    let (root_env, addr_env) = (ROOT_ENV, ADDR_ENV);
    let (route_imports, route_setup, router_call) = match nvtx_routes {
        NvtxRoutes::Enabled => (
            quote! {
                use quent_query_engine_server::analyzer_service_router_with_routes;
                use nvtx_server::{import_context_events, routes as nvtx_routes};
            },
            quote! {
                let nvtx_root = root.clone();
                let nvtx_importer = move |id: uuid::Uuid| import_context_events(&nvtx_root, id);
            },
            quote! {
                analyzer_service_router_with_routes::<Analyzer>(
                    Box::new(importer),
                    Box::new(lister),
                    None,
                    nvtx_routes(Box::new(nvtx_importer)),
                )
            },
        ),
        NvtxRoutes::Disabled => (
            quote! {
                use quent_query_engine_server::analyzer_service_router;
            },
            quote! {},
            quote! {
                analyzer_service_router::<Analyzer>(Box::new(importer), Box::new(lister), None)
            },
        ),
    };
    let tokens = quote! {
        use std::net::SocketAddr;
        use std::path::PathBuf;

        use quent_query_engine_analyzer::ui::QuentViewer;
        use quent_query_engine_server::analyzer_cache::index_query_engines;
        #route_imports
        use #analyzer_crate::Viewer;

        type Analyzer = <Viewer as QuentViewer>::Analyzer;

        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            let root = PathBuf::from(std::env::var(#root_env)?);
            let addr: SocketAddr = std::env::var(#addr_env)?.parse()?;

            let import_root = root.clone();
            let importer = move |id: uuid::Uuid| {
                Ok(<Viewer as QuentViewer>::import_events(&import_root.join(id.to_string()))?)
            };
            let lister_root = root.clone();
            let lister = move || index_query_engines(&lister_root);
            #route_setup

            let router = #router_call?;

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, router.into_make_service()).await?;
            Ok(())
        }
    };
    let file = syn::parse2(tokens).expect("generated wrapper main.rs is valid Rust");
    format!(
        "// Generated by quent-open. Do not edit.\n{}",
        prettyplease::unparse(&file)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{GitPin, ViewerSpec};

    fn spec() -> ViewerSpec {
        ViewerSpec {
            analyzer_package: "quent-simulator-analyzer".into(),
            quent: GitPin {
                remote: "https://example.com/quent".into(),
                commit: "quentcommit".into(),
            },
            analyzer: GitPin {
                remote: "https://example.com/analyzer".into(),
                commit: "analyzercommit".into(),
            },
        }
    }

    #[test]
    fn cargo_toml_pins_quent_and_analyzer() {
        let manifest: toml::Value =
            toml::from_str(&cargo_toml(&spec(), IO_PACKAGE, NvtxRoutes::Enabled)).unwrap();
        assert!(manifest.get("workspace").is_some(), "standalone workspace");
        let deps = &manifest["dependencies"];
        let server = &deps["quent-query-engine-server"];
        assert_eq!(server["git"].as_str().unwrap(), "https://example.com/quent");
        assert_eq!(server["rev"].as_str().unwrap(), "quentcommit");
        assert_eq!(server["features"][0].as_str().unwrap(), "ui");
        assert_eq!(deps["nvtx-server"]["rev"].as_str().unwrap(), "quentcommit");
        // The exporter enables all formats so the analyzer detects the artifact's format at runtime.
        let exporter_features = deps["quent-io"]["features"].as_array().unwrap();
        for format in ["ndjson", "msgpack", "postcard"] {
            assert!(exporter_features.iter().any(|f| f.as_str() == Some(format)));
        }
        let analyzer = &deps["quent-simulator-analyzer"];
        assert_eq!(
            analyzer["git"].as_str().unwrap(),
            "https://example.com/analyzer"
        );
        assert_eq!(analyzer["rev"].as_str().unwrap(), "analyzercommit");
    }

    #[test]
    fn cargo_toml_supports_the_legacy_io_package() {
        // Artifacts pinned to quent revisions predating the `quent-exporter` →
        // `quent-io` rename also predate the NVTX server package.
        let manifest: toml::Value = toml::from_str(&cargo_toml(
            &spec(),
            LEGACY_IO_PACKAGE,
            NvtxRoutes::Disabled,
        ))
        .unwrap();
        let deps = &manifest["dependencies"];
        assert!(deps.get("quent-io").is_none());
        let exporter = &deps["quent-exporter"];
        assert!(deps.get("nvtx-server").is_none());
        assert_eq!(
            exporter["git"].as_str().unwrap(),
            "https://example.com/quent"
        );
        assert_eq!(exporter["rev"].as_str().unwrap(), "quentcommit");
        let features = exporter["features"].as_array().unwrap();
        for format in ["ndjson", "msgpack", "postcard"] {
            assert!(features.iter().any(|f| f.as_str() == Some(format)));
        }
    }

    #[test]
    fn cargo_toml_can_disable_nvtx_with_the_current_io_package() {
        let manifest: toml::Value =
            toml::from_str(&cargo_toml(&spec(), IO_PACKAGE, NvtxRoutes::Disabled)).unwrap();
        assert!(manifest["dependencies"].get(NVTX_SERVER_PACKAGE).is_none());
    }

    #[test]
    fn main_rs_wires_the_nvtx_viewer() {
        let main = main_rs(&spec(), NvtxRoutes::Enabled);
        assert!(main.contains("use quent_simulator_analyzer::Viewer;"));
        assert!(main.contains("import_context_events"));
        assert!(main.contains("analyzer_service_router_with_routes"));
        assert!(main.contains("QUENT_OPEN_ADDR")); // bind address is configurable
    }

    #[test]
    fn main_rs_without_nvtx_uses_the_legacy_router() {
        let main = main_rs(&spec(), NvtxRoutes::Disabled);
        assert!(main.contains("use quent_query_engine_server::analyzer_service_router;"));
        assert!(!main.contains("nvtx_server"));
        assert!(!main.contains("analyzer_service_router_with_routes"));
    }

    #[test]
    fn main_rs_imports_each_context_once_in_both_modes() {
        for nvtx_routes in [NvtxRoutes::Enabled, NvtxRoutes::Disabled] {
            let main = main_rs(&spec(), nvtx_routes);
            assert_eq!(
                main.matches("<Viewer as QuentViewer>::import_events")
                    .count(),
                1
            );
            assert_eq!(main.matches("&import_root.join(id.to_string())").count(), 1);
        }
    }
}
