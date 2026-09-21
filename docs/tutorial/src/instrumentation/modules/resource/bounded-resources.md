# Bounded resources

`known-bounds: true` states that a capacity has an explicit bound. An event or
FSM state attribute marked with `sets-resource-bounds: true` carries the
generated bounds record whenever that limit changes.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/bounded-resource/model.yaml}}
```

The resource declaration generates both `MemoryUsage` and `MemoryBounds`. Both
records use `u64` for the `bytes` field. Resource declarations do not currently
support selecting another numeric width.

## Instrumentation API

The memory entity publishes its current bound. The task separately records how
much of that capacity it claims.

```rust
{{#include ../../../../../../crates/yaml/examples/bounded-resource/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/bounded-resource/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/bounded-resource/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A known bound records the available capacity separately from resource usage.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="12">
  <h2>Check yourself</h2>
  <fieldset data-answer="a" data-explanation="A capacity with known bounds generates a bounds record in addition to its usage record.">
    <legend>What additional generated record comes from <code>known-bounds: true</code>?</legend>
    <label><input type="radio" name="q12a" value="a"> <code>MemoryBounds</code></label>
    <label><input type="radio" name="q12a" value="b"> <code>TaskBounds</code></label>
    <label><input type="radio" name="q12a" value="c"> <code>MemoryContext</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="b" data-explanation="sets-resource-bounds marks the event attribute that carries the generated bounds record.">
    <legend>What does <code>sets-resource-bounds: true</code> identify?</legend>
    <label><input type="radio" name="q12b" value="a"> The resource usage reference</label>
    <label><input type="radio" name="q12b" value="b"> The event attribute carrying the new bounds</label>
    <label><input type="radio" name="q12b" value="c"> The FSM's initial state</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/bounded-resource/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/bounded-resource/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/bounded-resource/main.py
