# Semantic Modules

Semantic modules, or mods, are curated vertical slices of Quent's stack. A mod
adds reusable meaning and constraints to basic schema elements, then lets
instrumentation, analysis, and user-interface tooling interpret those elements
consistently. Quent currently develops and curates these modules as integrated
parts of the telemetry stack. Mods are the primary way Quent is intended to
evolve, incrementally adding capabilities without requiring corresponding
changes to the core framework.

For example, the [Finite-State Machine](finite-state-machine/index.md) mod
describes the allowed order of an entity's events, while the
[Resource](resource/index.md) mod describes capacity and usage. An Application
Event Schema can combine mods to express the behavior relevant to that
application.
