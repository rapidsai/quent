# C++ Integration Example

## Structure

```
instrumentation/   Rust model definitions and event types
bridge/            CXX bridge generation (build.rs produces gen/ and include/)
cpp/               C++ application using generated headers
```

After building, `bridge/gen/` contains generated Rust FFI modules and
`bridge/include/` contains the C++ headers included by `cpp/src/main.cpp`.

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
