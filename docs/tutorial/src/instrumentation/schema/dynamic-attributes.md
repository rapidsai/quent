# Dynamic attributes

Most event attributes have names and types fixed by the schema. This lets the
generated instrumentation API check their use before the application runs.

The `dynamic` type provides a typed container whose keys and value types are
chosen when the event is emitted. It is useful when the available details
cannot be known while writing the schema. The generated event method still
requires a dynamic attribute container, but the schema does not check the keys
or types placed inside it. Prefer regular attributes for stable event data.

## YAML model

```yaml
{{#include ../../../../../crates/yaml/examples/dynamic-attributes/model.yaml}}
```

The schema declares one dynamic container on each event. It does not declare
the individual keys that the containers will hold.

## Instrumentation API

```rust
{{#include ../../../../../crates/yaml/examples/dynamic-attributes/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../experimental/vibe/codegen/cpp/example/tutorial/dynamic-attributes/main.cpp:6:}}
```

```python
{{#include ../../../../../experimental/vibe/codegen/python/example/tutorial/dynamic-attributes/main.py:4:}}
```

The application adds a string and an integer to the `started` event, then a
boolean and an integer to the `ended` event. Each value retains its runtime
type. The target-language API provides wrappers or conversions for selecting an
exact numeric type. Dynamic attributes also support null values, which do not
retain an intended value type.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>Use dynamic attributes for event details that cannot be defined in advance.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="dynamic">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="Dynamic attribute keys are chosen when the event is emitted.">
    <legend>Where are keys such as <code>queue</code> and <code>cached</code> defined?</legend>
    <label><input type="radio" name="q-dynamic-a" value="a"> In the YAML schema</label>
    <label><input type="radio" name="q-dynamic-a" value="b"> When the event is emitted</label>
    <label><input type="radio" name="q-dynamic-a" value="c"> By the event exporter</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="a" data-explanation="The keys and their value types gain runtime flexibility but are not checked against the schema.">
    <legend>What is the main tradeoff of using <code>dynamic</code>?</legend>
    <label><input type="radio" name="q-dynamic-b" value="a"> Its contents are flexible but are not checked against the schema</label>
    <label><input type="radio" name="q-dynamic-b" value="b"> It can contain only text values</label>
    <label><input type="radio" name="q-dynamic-b" value="c"> It prevents an event from having regular attributes</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/dynamic-attributes/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/dynamic-attributes/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/dynamic-attributes/main.py
