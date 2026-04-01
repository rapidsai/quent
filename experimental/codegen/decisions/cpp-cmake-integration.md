# Decision: C++ CMake Integration

## Context

C++ applications need to integrate model-generated instrumentation code into
their build. The codegen produces CXX bridge modules (Rust) that CXX turns
into C++ headers. The question is how this integrates into a CMake-based C++
project.

## Decision

Ship a CMake module (`quent_add_model`) alongside the `quent-codegen` CLI.
Users provide only a model definition file (YAML/JSON). No generated code is
checked in — everything is generated at build time into `CMAKE_BINARY_DIR`.

## Usage

```cmake
include(quent_telemetry)
quent_add_model(
    MODEL ${CMAKE_SOURCE_DIR}/model.yaml
    TARGET quent_telemetry
    NAMESPACE myapp::telemetry
)
target_link_libraries(my_engine PRIVATE quent_telemetry)
```

## What the CMake module does

1. Calls `quent-codegen` to generate a Rust crate with `#[cxx::bridge]`
   modules from `model.yaml`
2. Uses Corrosion to compile the generated Rust crate
3. CXX generates C++ headers from the bridge definitions
4. Exports a CMake target with correct include paths and link dependencies

Regeneration happens automatically when `model.yaml` changes (via CMake
dependency tracking on the model file).

## What the user's repo contains

```
model.yaml          # model definition (checked in)
CMakeLists.txt      # includes quent, calls quent_add_model
src/
  engine.cpp        # uses generated headers
```

No generated code. No Cargo.toml. The CMake module handles all Rust/CXX
setup internally.

## quent-codegen CLI

```bash
quent-codegen \
  --input model.yaml \
  --target cxx \
  --output ${CMAKE_BINARY_DIR}/telemetry/ \
  --namespace myapp::telemetry \
  --cmake-target quent_telemetry
```

Outputs a self-contained directory with `Cargo.toml`, generated Rust source,
and a `CMakeLists.txt` that Corrosion consumes.

## Installation

`quent-codegen` is installed via `cargo install`, a package manager, or a
pre-built binary. The CMake module (`quent_telemetry.cmake`) is installed
alongside it and found via `CMAKE_MODULE_PATH` or `find_package`.

## Model input format

The model YAML/JSON schema matches the `ModelBuilder` structure from
`quent-model`. The same `ModelBuilder` types are used regardless of whether
the model comes from Rust derive macros or a YAML file.

For Rust projects, the derive macros populate `ModelBuilder` in-process
(via `build.rs`). For C++ projects, the YAML file is parsed into the same
`ModelBuilder` by the codegen CLI.

## Rationale

- Users should not be required to check in generated code
- Users should not need to write Rust or manage Cargo.toml
- The model definition (YAML) is the only input the user maintains
- The Rust toolchain is already required (CXX bridge links Rust runtime)
- CMake is the standard C++ build system for the target audience
- Corrosion is a proven CMake-Cargo bridge (used by Sirius)
