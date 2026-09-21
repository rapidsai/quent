# Repeated events

Events are `once` by default. Set `multi: true` when one entity instance may
emit the same event repeatedly.

## YAML model

```yaml
{{#include ../../../../../crates/yaml/examples/repeated-events/model.yaml}}
```

## Instrumentation API

The same `Task` handle emits `progress` more than once. Repeating `started` or
`ended` on that handle would return an error.

```rust
{{#include ../../../../../crates/yaml/examples/repeated-events/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../experimental/vibe/codegen/cpp/example/tutorial/repeated-events/main.cpp:6:}}
```

```python
{{#include ../../../../../experimental/vibe/codegen/python/example/tutorial/repeated-events/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p><code>multi: true</code> permits an event to be emitted more than once for an entity.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="03">
  <h2>Check yourself</h2>
  <fieldset data-answer="c" data-explanation="multi: true gives the event multi cardinality for each entity instance.">
    <legend>Why can <code>progress</code> be called repeatedly?</legend>
    <label><input type="radio" name="q03a" value="a"> It has a <code>u64</code> attribute</label>
    <label><input type="radio" name="q03a" value="b"> It belongs to <code>Task</code></label>
    <label><input type="radio" name="q03a" value="c"> It declares <code>multi: true</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="a" data-explanation="started uses the default once cardinality, so a second emission on the same handle returns an error.">
    <legend>What happens when <code>started</code> is emitted twice on one handle?</legend>
    <label><input type="radio" name="q03b" value="a"> The second call returns an error</label>
    <label><input type="radio" name="q03b" value="b"> Both events are emitted</label>
    <label><input type="radio" name="q03b" value="c"> The process exits</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/repeated-events/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/repeated-events/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/repeated-events/main.py
