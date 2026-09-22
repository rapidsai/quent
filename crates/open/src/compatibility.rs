// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Select the generated wrapper contract for a pinned Quent revision.

use std::path::Path;

use proc_macro2::TokenStream;
use quote::quote;

use crate::error::Result;
use crate::revision;
use crate::spec::ViewerSpec;

/// Cargo package of Quent's current I/O crate.
pub(crate) const IO_PACKAGE: &str = "quent-io";
/// Cargo package used before [`IO_PACKAGE`] was introduced.
pub(crate) const LEGACY_IO_PACKAGE: &str = "quent-exporter";
/// Cargo package that provides the optional NVTX HTTP routes.
pub(crate) const NVTX_SERVER_PACKAGE: &str = "nvtx-server";

/// Boundary whose descendants provide the `quent-io` package.
///
/// Introduced by [commit `aa1e9b1`](https://github.com/rapidsai/quent/commit/aa1e9b1b394f5f978215b69cd5c526291c4b4723).
const IO_PACKAGE_BOUNDARY: &str = "aa1e9b1b394f5f978215b69cd5c526291c4b4723";
/// Boundary whose descendants provide NVTX routes and the extensible analyzer router.
///
/// Introduced by [commit `f40e69c`](https://github.com/rapidsai/quent/commit/f40e69c2d4405c765c6270221e2a58e58ef704a6).
const NVTX_ROUTES_BOUNDARY: &str = "f40e69c2d4405c765c6270221e2a58e58ef704a6";
/// Boundary whose strict descendants provide context-inventory indexing.
///
/// Introduced after [commit `cee18e0`](https://github.com/rapidsai/quent/commit/cee18e047c5407dc91b8d9e6e150892444775bd1).
const CONTEXT_INVENTORY_PREDECESSOR: &str = "cee18e047c5407dc91b8d9e6e150892444775bd1";

/// First published commit providing model-owned viewer composition.
///
/// This is a containing commit, not a predecessor on main: unrelated mainline
/// changes must continue to use the historical wrapper. Preserve this SHA for
/// branch-pinned artifacts. If merged by squash, add the upstream merge SHA as
/// another containing boundary instead of replacing this one.
pub(crate) const MODEL_VIEWER_BOUNDARY: &str = "a183943254961fa1ead4dca59bc1cbade6f24558";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewerContract {
    Legacy { nvtx: bool },
    Model,
}

impl ViewerContract {
    pub(crate) fn needs_nvtx_dependency(self) -> bool {
        self == Self::Legacy { nvtx: true }
    }

    pub(crate) fn code(self) -> ViewerCode {
        if self == Self::Model {
            return ViewerCode {
                imports: quote! {
                    use quent_query_engine_server::model_viewer_router;
                },
                setup: quote! {},
                router: quote! {
                    model_viewer_router::<Viewer>(Box::new(importer), Box::new(lister), None)
                },
            };
        }
        if self.needs_nvtx_dependency() {
            ViewerCode {
                imports: quote! {
                    use quent_query_engine_server::analyzer_service_router_with_routes;
                    use nvtx_server::{import_context_events, routes as nvtx_routes};
                },
                setup: quote! {
                    let nvtx_root = root.clone();
                    let nvtx_importer = move |id: uuid::Uuid| {
                        import_context_events(&nvtx_root, id)
                    };
                },
                router: quote! {
                    analyzer_service_router_with_routes::<Analyzer>(
                        Box::new(importer),
                        Box::new(lister),
                        None,
                        nvtx_routes(Box::new(nvtx_importer)),
                    )
                },
            }
        } else {
            ViewerCode {
                imports: quote! {
                    use quent_query_engine_server::analyzer_service_router;
                },
                setup: quote! {},
                router: quote! {
                    analyzer_service_router::<Analyzer>(Box::new(importer), Box::new(lister), None)
                },
            }
        }
    }
}

pub(crate) struct ViewerCode {
    pub(crate) imports: TokenStream,
    pub(crate) setup: TokenStream,
    pub(crate) router: TokenStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContextIndexing {
    QueryEngines,
    ContextInventory,
}

impl ContextIndexing {
    pub(crate) fn code(self) -> ContextIndexingCode {
        match self {
            Self::QueryEngines => ContextIndexingCode {
                import: quote! {
                    use quent_query_engine_server::analyzer_cache::index_query_engines;
                },
                lister: quote! {
                    let lister_root = root.clone();
                    let lister = move || index_query_engines(&lister_root);
                },
            },
            Self::ContextInventory => ContextIndexingCode {
                import: quote! {
                    use quent_analyzer::context::index_contexts;
                },
                lister: quote! {
                    let lister_root = root.clone();
                    let lister = move || {
                        index_contexts(&lister_root, |context_dir| {
                            Ok::<_, quent_query_engine_server::error::ServerError>(
                                <Viewer as QuentViewer>::context_inventory(context_dir)?,
                            )
                        })
                    };
                },
            },
        }
    }
}

pub(crate) struct ContextIndexingCode {
    pub(crate) import: TokenStream,
    pub(crate) lister: TokenStream,
}

pub(crate) struct WrapperCompatibility {
    pub(crate) viewer: ViewerContract,
    pub(crate) io_package: &'static str,
    pub(crate) context_indexing: ContextIndexing,
}

impl WrapperCompatibility {
    pub(crate) async fn resolve(repository: &Path, spec: &ViewerSpec) -> Result<Self> {
        let revision = revision::PinnedRevision::fetch(repository, &spec.quent).await?;
        let viewer = if revision.contains(MODEL_VIEWER_BOUNDARY).await? {
            ViewerContract::Model
        } else {
            ViewerContract::Legacy {
                nvtx: revision.contains(NVTX_ROUTES_BOUNDARY).await?,
            }
        };
        let io_package = if revision.contains(IO_PACKAGE_BOUNDARY).await? {
            IO_PACKAGE
        } else {
            LEGACY_IO_PACKAGE
        };
        let context_indexing = if revision
            .is_strict_descendant_of(CONTEXT_INVENTORY_PREDECESSOR)
            .await?
        {
            ContextIndexing::ContextInventory
        } else {
            ContextIndexing::QueryEngines
        };
        Ok(Self {
            viewer,
            io_package,
            context_indexing,
        })
    }
}
