# Records

A record groups related fields into a named, reusable type. Event attributes
can use a record name wherever they can use a scalar type.

## YAML model

```yaml
{{#include ../../../../../crates/yaml/examples/records/model.yaml}}
```

## Instrumentation API

The generated API represents `WorkResult` as a target-language record type.
Both `Task` and `Batch` accept that type when emitting `ended`.

```rust
{{#include ../../../../../crates/yaml/examples/records/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../experimental/vibe/codegen/cpp/example/tutorial/records/main.cpp:6:}}
```

```python
{{#include ../../../../../experimental/vibe/codegen/python/example/tutorial/records/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A named record provides one reusable structure for event attributes.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="04">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="An event attribute can use a record name wherever it can use a scalar type.">
    <legend>What type do the <code>Task</code> and <code>Batch</code> <code>ended</code> events expect for <code>result</code>?</legend>
    <label><input type="radio" name="q04a" value="a"> <code>bool</code></label>
    <label><input type="radio" name="q04a" value="b"> <code>WorkResult</code></label>
    <label><input type="radio" name="q04a" value="c"> <code>Task</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="a" data-explanation="Records give a group of related fields one reusable, generated type.">
    <legend>Why declare a record instead of repeating its fields?</legend>
    <label><input type="radio" name="q04b" value="a"> To reuse one named field group</label>
    <label><input type="radio" name="q04b" value="b"> To make the event repeatable</label>
    <label><input type="radio" name="q04b" value="c"> To choose an exporter</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/records/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/records/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/records/main.py
