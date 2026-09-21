# Reference Target

The Reference Target semantic module constrains an entity reference to a
specific entity type. A targeted `ref` links one entity to another entity of a
declared type. Here, the task's `started` event records which `Worker` runs it.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/entity-references/model.yaml}}
```

## Instrumentation API

The generated `started` method only accepts the target-language representation
of a reference to a `Worker`.

```rust
{{#include ../../../../../../crates/yaml/examples/entity-references/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/entity-references/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/entity-references/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>An entity reference identifies a specific entity and preserves its type.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="06">
  <h2>Check yourself</h2>
  <fieldset data-answer="c" data-explanation="The reference target is part of the generated type, so started requires a Worker reference.">
    <legend>Which entity type may the <code>worker</code> attribute target?</legend>
    <label><input type="radio" name="q06a" value="a"> Any entity</label>
    <label><input type="radio" name="q06a" value="b"> Only <code>Task</code></label>
    <label><input type="radio" name="q06a" value="c"> Only <code>Worker</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="b" data-explanation="ref constrains the target type but does not add the tree-forming scope marker.">
    <legend>Does <code>ref: Worker</code> place <code>Task</code> under <code>Worker</code> in a hierarchy?</legend>
    <label><input type="radio" name="q06b" value="a"> Yes</label>
    <label><input type="radio" name="q06b" value="b"> No</label>
    <label><input type="radio" name="q06b" value="c"> Only when the event is <code>multi</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/entity-references/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/entity-references/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/entity-references/main.py
