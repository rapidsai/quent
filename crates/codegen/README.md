# Quent codegen

Validate an instrumentation package manifest and its YAML model:

```bash
pixi run cargo run -p quent-codegen -- check --manifest-path examples/readme/quent.toml
```

The manifest contains the model path, requested targets, and package metadata:

```toml
model = "model.yaml"
targets = ["rust"]

[package]
name = "my-instrumentation"
version = "0.1.0"
```

`check` defaults to `quent.toml` in the current directory. Relative model paths
are resolved from the manifest's directory; absolute paths are also accepted.
All fields are required, and unknown fields are errors. Only the `rust` target
is supported. The target list must be nonempty and contain no duplicates.
Package names must be nonempty and contain only ASCII letters, digits, hyphens,
or underscores. Package versions must be full semantic versions, such as
`0.1.0`.

Validation checks the manifest and the YAML parser's schema constraints. It
prints warnings to stderr without failing, and exits unsuccessfully on errors,
including missing files. It does not generate files, compile a package, or check
all target-specific generation requirements.

The manifest CLI currently supports only the `rust` target.
