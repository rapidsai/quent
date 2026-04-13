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

## Event output

The `ndjson` exporter in the above example writes one JSON object per line,
which is typically only useful for debugging and manual inspection. Production
deployments can use the MessagePack or Postcard exporters for lower overhead, or
stream to a centralized collector for distributed deployments, but to illustrate
the events stored, an example of the output is shown below:

```json
{"id":"019d...","timestamp":1712345678000000000,"data":{"Task":{"Transition":{"seq":0,"state":{"Queued":{"name":"query-42","queue":{"resource_id":"01a2...","capacity":{"depth":1}}}}}}}}
{"id":"019d...","timestamp":1712345678000100000,"data":{"Task":{"Transition":{"seq":1,"state":{"Running":{"thread":{"resource_id":"01b3...","capacity":{}}}}}}}}
{"id":"019d...","timestamp":1712345678000200000,"data":{"Task":{"Transition":{"seq":2,"state":"Exit"}}}}
```
