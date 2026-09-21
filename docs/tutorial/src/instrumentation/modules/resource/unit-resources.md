# Unit resources

`resource: true` declares an indivisible resource. This model places each
`Thread` under a `ThreadPool`, then places a running `Task` under the specific
thread it claims.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/unit-resource/model.yaml}}
```

The `ThreadUsage` record is generated automatically. It has no fields because a
unit resource is claimed as a whole.

## Instrumentation API

The generated API attaches the usage record to the scoped thread reference.

```rust
{{#include ../../../../../../crates/yaml/examples/unit-resource/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/unit-resource/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/unit-resource/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A unit resource represents one indivisible resource instance.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="10">
  <h2>Check yourself</h2>
  <fieldset data-answer="c" data-explanation="resource: true declares a unit resource that is claimed as one indivisible instance.">
    <legend>What does <code>resource: true</code> mean for <code>Thread</code>?</legend>
    <label><input type="radio" name="q10a" value="a"> It has unlimited capacity</label>
    <label><input type="radio" name="q10a" value="b"> It emits repeated events</label>
    <label><input type="radio" name="q10a" value="c"> It is an indivisible resource</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="a" data-explanation="Thread scopes to ThreadPool, and Task scopes to the Thread reference carrying ThreadUsage.">
    <legend>Which hierarchy does the model define?</legend>
    <label><input type="radio" name="q10b" value="a"> <code>ThreadPool → Thread → Task</code></label>
    <label><input type="radio" name="q10b" value="b"> <code>Task → ThreadPool → Thread</code></label>
    <label><input type="radio" name="q10b" value="c"> No hierarchy</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/unit-resource/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/unit-resource/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/unit-resource/main.py
