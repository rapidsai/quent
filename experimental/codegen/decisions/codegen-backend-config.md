# Decision: Codegen Backend Configuration

## Context

The model stores canonical names (snake_case derived from Rust struct names).
Target languages have different naming conventions, and even within a single
language, different codebases follow different conventions. The codegen backends
need to be configurable per target.

## Decision

Each codegen backend accepts a configuration struct that controls
language-specific output conventions. The model definition is unaware of these
settings.

## Example

```rust
quent_codegen::emit_cpp(
    &model,
    CppOptions {
        method_case: SnakeCase,
        class_case: PascalCase,
        struct_case: PascalCase,
        namespace: "myapp::telemetry",
        output_dir: "generated/cpp",
        header_extension: "hpp",
        // ...
    },
);
```

## Scope

Backend configuration covers cosmetic and structural choices that do not affect
semantics:

- **Naming conventions**: case style for methods, classes, structs, enum
  variants, constants
- **Output layout**: directory structure, file naming, header extension
- **Language idioms**: namespace/package, include guards vs. `#pragma once`,
  visibility modifiers

It does not cover:

- **Model structure**: what FSMs, states, resources exist (that's the model)
- **Canonical names**: the model's snake_case names are the source; backends
  transform them

## Rationale

- Different C++ codebases follow different naming conventions. Forcing one
  style would require manual renaming or wrapper layers.
- The same applies to future backends (Java package names, Python module
  structure, etc.).
- Keeping configuration out of the model definition means the model is
  target-agnostic. Adding or reconfiguring a backend does not change the model.
- Configuration is per invocation of the codegen backend, so the same model can
  produce differently-styled output for different consumers if needed.
