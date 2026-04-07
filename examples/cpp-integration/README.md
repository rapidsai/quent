# C++ Integration Example

## Structure

| Directory | Description | Generated |
|---|---|---|
| `instrumentation/` | Rust model definitions and event types | No |
| `bridge/` | CXX bridge crate with `build.rs` | No |
| `bridge/gen/` | Rust FFI modules | Yes |
| `bridge/include/` | C++ headers for `main.cpp` | Yes |
| `cpp/` | C++ application | No |

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

This produces an ndjson file in `cpp/data/`. Verify it contains events:

```bash
cat data/*.ndjson | head -5
```

Each line is a JSON object with an `id`, `timestamp`, and `data` field
representing an FSM transition or entity event.
