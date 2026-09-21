# Minimal model

Every model declares the YAML format version and a model name. This model has
one entity type, `Task`, with two events. Events are emitted at most once per
entity instance unless the model says otherwise.

## YAML model

```yaml
{{#include ../../../../../crates/yaml/examples/minimal-model/model.yaml}}
```

## Instrumentation API

The context provides an observer for each entity type. Calling `.handle()` on
an observer creates a handle for a new entity instance and assigns it a fresh
UUID. The handle exposes one method per event.

Every context is created with an exporter, which determines where emitted
events go. These examples use the no-op exporter, which discards every event.
It keeps the examples focused on the generated API without creating files or
starting another service. Applications replace it with an exporter that stores
or sends their events.

```rust
{{#include ../../../../../crates/yaml/examples/minimal-model/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../experimental/vibe/codegen/cpp/example/tutorial/minimal-model/main.cpp:6:}}
```

```python
{{#include ../../../../../experimental/vibe/codegen/python/example/tutorial/minimal-model/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>The two events have no declared ordering constraint, so either event may be emitted first.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="01">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="Events are once by default. Use multi: true when repetition is part of the model.">
    <legend>How often can <code>started</code> be emitted for one <code>Task</code> handle?</legend>
    <label><input type="radio" name="q01a" value="a"> Any number of times</label>
    <label><input type="radio" name="q01a" value="b"> Once</label>
    <label><input type="radio" name="q01a" value="c"> Once for the entire process</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="A handle identifies one entity instance and emits that instance's events.">
    <legend>What does <code>task</code> represent in the application?</legend>
    <label><input type="radio" name="q01b" value="a"> The complete model</label>
    <label><input type="radio" name="q01b" value="b"> Every <code>Task</code> entity</label>
    <label><input type="radio" name="q01b" value="c"> One <code>Task</code> entity instance</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/minimal-model/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/minimal-model/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/minimal-model/main.py
