# Entity references

An attribute of type `ref` identifies another entity instance by its UUID. The
reference is type-erased: the generated API knows that it is an entity
reference, but does not restrict which entity type it targets. This is useful
when any kind of entity is a valid target.

## YAML model

```yaml
{{#include ../../../../../crates/yaml/examples/untyped-entity-references/model.yaml}}
```

## Instrumentation API

Every entity handle exposes its identity as a type-erased reference. Here, the
task's `started` event receives a reference to the `Worker` instance.

```rust
{{#include ../../../../../crates/yaml/examples/untyped-entity-references/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../experimental/vibe/codegen/cpp/example/tutorial/untyped-entity-references/main.cpp:6:}}
```

```python
{{#include ../../../../../experimental/vibe/codegen/python/example/tutorial/untyped-entity-references/main.py:4:}}
```

When an attribute must target a particular entity type, the
[Reference Target](../modules/reference-target/index.md) semantic module adds
that restriction to the generated API.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A type-erased reference preserves an entity's identity without restricting its entity type.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="05">
  <h2>Check yourself</h2>
  <fieldset data-answer="a" data-explanation="A reference identifies its target entity instance by UUID.">
    <legend>What does a type-erased entity reference preserve?</legend>
    <label><input type="radio" name="q05a" value="a"> The target entity's identity (UUID)</label>
    <label><input type="radio" name="q05a" value="b"> A copy of every target event</label>
    <label><input type="radio" name="q05a" value="c"> The target entity's exporter</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="b" data-explanation="A bare ref accepts references to any entity type. Reference Target adds a target-type restriction.">
    <legend>Does a bare <code>ref</code> require its target to be a <code>Worker</code>?</legend>
    <label><input type="radio" name="q05b" value="a"> Yes</label>
    <label><input type="radio" name="q05b" value="b"> No</label>
    <label><input type="radio" name="q05b" value="c"> Only for repeated events</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/untyped-entity-references/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/untyped-entity-references/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/untyped-entity-references/main.py
