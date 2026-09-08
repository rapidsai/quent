# Legacy Python Integration Example

This example exposes the same Rust model used by `examples/legacy/readme` and
`examples/legacy/cpp-integration` as a Python extension module generated with
PyO3.

Build and install the extension into the active Python environment:

```bash
cd examples/legacy/python-integration
maturin develop
python main.py
```

The generated module is named `quent_readme`. Its schema comes from the normal
Rust `model!` / `entity!` / `fsm!` definitions in `examples/legacy/readme`; the
bridge crate only asks `quent-codegen` to emit PyO3 wrappers from
`AppModel::build`. As well as generating python bindings, the build script also
emits type stubs in the generated wheel.

Contexts accept typed exporter options. Filesystem exporters require an output
path:

```python
from pathlib import Path

import quent_readme as quent

context = quent.Context(quent.ExporterOptions.ndjson(Path("./events")))
```
