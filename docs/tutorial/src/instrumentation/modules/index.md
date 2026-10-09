# Semantic Modules

Semantic modules, or mods, are curated vertical slices of Quent's stack. A mod
adds reusable meaning and constraints to basic schema elements, then lets
instrumentation, analysis, and user-interface tooling interpret (the _semantics_
of) those elements consistently. Quent currently develops and curates these
modules as integrated parts of the telemetry stack. Mods are the primary way
Quent is intended to evolve, incrementally adding capabilities without requiring
major changes to the core framework.

The YAML-based DSL provides additional syntax that makes it easy to start
leveraging these semantic modules. An optional, more detailed explanation of how
this relates to YAML-based schema capture follows below. It is not necessary to
understand these details in order to benefit from semantic modules. Feel free to
skip to the next lesson.

## YAML Syntax for Semantic Modules

Semantic modules express their additional rules as opaque annotations on schema
elements. They may also add ordinary schema elements such as records, entities,
events, or fields. As described above, components in the various layers of Quent
interpret only annotations they understand, so adding support for new semantics
in one component does not necessarily affect other components.

For example, exporters and importers in the event I/O layer typically serialize
event data without interpreting its semantics. At the same time, the
code-generation layer can generate instrumentation APIs that, e.g. through
leveraging the type system of the target language, enforce certain rules about
events, preventing applications from emitting events that violate the module's
semantics.

Necessary data for semantic modules can be added to the schema directly through
YAML. For example, the expanded schema form of a small FSM (explained in more
detail in the [Finite-State Machine lesson](finite-state-machine/index.md))
contains state events and an opaque constraint describing its transitions:

```yaml
quent: '0.1.0'
model: query

entities:
  Query:
    constraints:
      quent.fsm.v0.1.0: '{"initial_state":"queued","transitions":[{"source":"queued","target":"running"}]}'
    events:
      queued:
        attributes:
          seq: u16 # required by the FSM constraint
      running:
        attributes:
          seq: u16
```

This is not very readable and it adds a lot of noisy details. To express these
semantics more concisely, the YAML-based DSL provides module-specific syntax.

For the example above, we can also write it as:

```yaml
quent: '0.1.0'
model: query

fsms:
  Query:
    states:
      queued:
        initial: true
        to: [running]
      running: {}
```

Thus, the YAML-based DSL parser will often provide syntactic sugar for semantic
modules. This additional syntax _related_ to semantic modules can appear at
different levels of the YAML tree. It _typically_ starts with a short key
corresponding to the name of the semantic module. For example:

- [Finite-State Machine](finite-state-machine/index.md) uses `fsms: ...` at the
  model root.
- [Resource](resource/index.md) uses `resource: ...` on an entity.
- [Operating System](operating-system/index.md) uses `os: ...` as a field type.

Its value and any nested fields are specific to that mod, and are documented in
more detail in the [YAML
reference](https://github.com/rapidsai/quent/blob/main/crates/yaml/README.md) as
well as the next few lessons in this chapter.

This type of syntactic sugar over plain schema elements typically provides a
concise way to:

- declare certain (potentially relatively complicated) event semantics
  - e.g. that events must be emitted in a sequence specified by some
    [Finite-State Machine](finite-state-machine/index.md)
  - e.g. that an entity represents something that provides certain
    [Resource](resource/index.md) to other entities
- add a large number of standardized elements (typically records)
  - e.g. to add schema records representing events defined by
    [NVTX](https://nvidia.github.io/NVTX/)

Semantic modules sometimes depend on each other. One piece of YAML syntax can
therefore add annotations from multiple mods to a schema element.
