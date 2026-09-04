# Experimental schema binding generators

This isolated Cargo workspace generates CXX and PyO3 bindings directly from a
`quent_schema::Schema`. It does not depend on `quent-model` or
`quent-model-macros`.

```sh
cargo test --manifest-path experimental/vibe/codegen/Cargo.toml --workspace
```

The target-language examples under `cpp/example` and `python/example` define
the intended public API. Both examples bind directly to the schema-generated
instrumentation in `examples/readme` and exercise once-event enforcement at
runtime.

## Git dependencies

An external project can consume either generator directly from the Quent Git
repository even though this is a nested workspace. Pin all Quent build
dependencies to one revision so they use the same `Schema` package identity:

```toml
[build-dependencies]
quent-schema-codegen-cpp = {
  git = "https://github.com/rapidsai/quent.git",
  rev = "<commit>",
}
quent-yaml = {
  git = "https://github.com/rapidsai/quent.git",
  rev = "<commit>",
}
```

The generator crates also re-export `quent_schema` for callers that construct
schemas directly. Git dependencies are supported despite `publish = false`;
these experimental crates are not published to crates.io.

## Generated bridge dependencies

A CXX bridge crate directly depends on `cxx`, the generated instrumentation
crate, `quent-instrumentation`, and `quent-dynamic-attributes`. A PyO3 bridge
uses the same dependencies with `pyo3` in place of `cxx`. Add `quent-io` only
when generating exporter constructors.

Exporter constructors are opt-in. Enable the corresponding
`quent-instrumentation` feature for each selected constructor:

| Generator option | Cargo feature |
| --- | --- |
| `ndjson` | `io-ndjson` |
| `msgpack` | `io-msgpack` |
| `postcard` | `io-postcard` |
| `collector` | `io-collector` |

These exporters also require instrumentation generated with
`quent_instrumentation_build::Options::serde`. The default generator options
expose only the no-op exporter and require none of these features.

The Rust dependency paths are configurable through each generator's `Options`
for projects that rename Cargo dependencies.
