# C++ Integration Example

Demonstrates the full pipeline from Rust model definition to C++ API.

## Model

Defined in `instrumentation/src/model.rs` — a job scheduler with:
- **Job**: entity (root resource group) with Submit/Complete events
- **ThreadPool**: entity (resource group) with Init event
- **Task**: FSM (Queued → Running → exit) using thread resources

## Structure

```
instrumentation/
  src/
    lib.rs                  crate root
    model.rs                model definitions (#[derive(Fsm/Entity/State)])
  examples/
    cpp_codegen.rs          runs codegen and prints generated CXX bridges
cpp/
  CMakeLists.txt            CMake build (Corrosion for Rust integration)
  src/
    main.cpp                C++ application using generated headers
```

## Pipeline

```
instrumentation/src/model.rs     Rust model definition
        |
        v
quent-codegen                    Generates CXX bridge Rust modules
        |
        v
CXX                              Generates C++ headers from bridge modules
        |
        v
cpp/src/main.cpp                 C++ application using generated headers
```

## View generated code

```bash
cargo run --example cpp_codegen -p quent-cpp-example-instrumentation
```

## Build the C++ example

Requires: cmake, a C++ compiler, and Rust toolchain (via pixi or manually).

```bash
pixi shell
cd examples/cpp-integration/cpp
cmake -B build
cmake --build build
```

## Target C++ API

```cpp
// Entity observer pattern
auto job_obs = telemetry::job::create_observer(*ctx);
job_obs->submit(job_id, telemetry::job::Submit{.name = "batch-42", .num_tasks = 4});

// FSM handle pattern
auto task = telemetry::task::create(*ctx, telemetry::task::Queued{
    .job_id = job_id, .name = "task-0",
});
task->running(telemetry::task::Running{.thread_resource_id = thread_0});
task->exit();
```
