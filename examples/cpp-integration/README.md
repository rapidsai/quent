# C++ Integration Example

Demonstrates using a Quent model from C++ via the CXX bridge.
Uses the same model as the [README example](../readme/src/lib.rs).

## Structure

```text
bridge/              CXX bridge crate with build.rs
  gen/               Generated Rust FFI modules
  include/           Generated C++ headers
cpp/
  src/main.cpp       C++ application exercising the model
```

## Build

Requires cmake, a C++ compiler, and Rust toolchain (via pixi or manually).

```bash
pixi shell
cd examples/cpp-integration/cpp
cmake -B build
cmake --build build
```

## Run

```bash
./build/example
```

This produces an ndjson file in `cpp/data/`.
Each line is a JSON object with `id`, `timestamp`, and `data` fields.
