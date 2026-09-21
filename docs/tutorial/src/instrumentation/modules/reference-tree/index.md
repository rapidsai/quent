# Reference Tree

The Reference Tree semantic module marks entity references that form
parent-child relationships. A `scope-ref` applies both the reference-target and
reference-tree constraints. The parser validates all scoped references together
as one tree.

## Why is a target type required?

A type-erased [`ref`](../../schema/entity-references.md) cannot form part of the
Reference Tree. A `scope-ref` must name its target entity type so Quent can
validate the complete tree when it processes the schema.

Without a declared target type, different instances of the same child entity
type could refer to different parent entity types at runtime. The generated
instrumentation API could then no longer guarantee that the resulting
relationships form the tree declared by the schema.

This gives analysis tools a preferred path from one root entity to every
related entity and its events. In the model below, a task points to the pipeline
that contains it. A user interface can open one pipeline and list its tasks,
while analysis can associate each task's events with that pipeline. Other tools
can use the same hierarchy for their own purposes.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/scoped-references/model.yaml}}
```

## Instrumentation API

The parent entity's handle provides the reference. The additional hierarchy
meaning belongs to the model and its constraints.

```rust
{{#include ../../../../../../crates/yaml/examples/scoped-references/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/scoped-references/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/scoped-references/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A scoped reference defines a parent relationship in a validated entity tree.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="07">
  <h2>Check yourself</h2>
  <fieldset data-answer="a" data-explanation="scope-ref adds the tree-forming constraint in addition to the target constraint.">
    <legend>What does <code>scope-ref</code> add beyond a normal targeted <code>ref</code>?</legend>
    <label><input type="radio" name="q07a" value="a"> A validated parent relationship</label>
    <label><input type="radio" name="q07a" value="b"> Repeated event cardinality</label>
    <label><input type="radio" name="q07a" value="c"> Resource bounds</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="The Pipeline handle produces the typed entity reference accepted by Task.started.">
    <legend>Which value is passed as the task's parent?</legend>
    <label><input type="radio" name="q07b" value="a"> The pipeline observer</label>
    <label><input type="radio" name="q07b" value="b"> The complete context</label>
    <label><input type="radio" name="q07b" value="c"> A reference from the pipeline handle</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/scoped-references/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/scoped-references/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/scoped-references/main.py
