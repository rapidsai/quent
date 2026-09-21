# Resource capacities

A resource can expose measured capacities. `occupancy` describes a quantity
held over the usage span, such as bytes of memory held while a task runs.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/resource-capacity/model.yaml}}
```

Declaring the `bytes` capacity generates a `MemoryUsage` record with a `bytes`
field.

## Instrumentation API

The task's reference to `Memory` carries the quantity it claims.

```rust
{{#include ../../../../../../crates/yaml/examples/resource-capacity/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/resource-capacity/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/resource-capacity/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>A capacity resource records the quantity of a resource used by an entity.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="11">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="Occupancy is a quantity held over a usage span, such as allocated bytes while running.">
    <legend>What does an <code>occupancy</code> capacity measure?</legend>
    <label><input type="radio" name="q11a" value="a"> A one-time event count</label>
    <label><input type="radio" name="q11a" value="b"> A quantity held during a usage span</label>
    <label><input type="radio" name="q11a" value="c"> A hierarchy depth</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="MemoryUsage is generated from Memory's capacities and declares the claimed bytes.">
    <legend>Where is the task's claimed byte quantity declared?</legend>
    <label><input type="radio" name="q11b" value="a"> <code>ResourceCapacityContext</code></label>
    <label><input type="radio" name="q11b" value="b"> The task UUID</label>
    <label><input type="radio" name="q11b" value="c"> <code>MemoryUsage</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/resource-capacity/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/resource-capacity/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/resource-capacity/main.py
