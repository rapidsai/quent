# Self-loops

A direct self-loop transitions from a state back to itself, such as
`running → running`. A state can also be part of an indirect cycle, such as
`running → paused → running`. Both forms allow states on the cycle to be
entered repeatedly, so their generated events have `multi` cardinality.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/fsm-self-loop/model.yaml}}
```

## Instrumentation API

The task first repeats `running` through its direct self-loop. It then follows
the indirect cycle through `paused` and back to `running` before entering
`completed` once.

```rust
{{#include ../../../../../../crates/yaml/examples/fsm-self-loop/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/fsm-self-loop/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/fsm-self-loop/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>Every state on a direct or indirect cycle has a repeatable state-entry event.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="09">
  <h2>Check yourself</h2>
  <fieldset data-answer="a" data-explanation="The running state names itself in its to list, creating a direct self-loop.">
    <legend>Which transition is a direct self-loop?</legend>
    <label><input type="radio" name="q09a" value="a"> <code>running → running</code></label>
    <label><input type="radio" name="q09a" value="b"> <code>running → paused</code></label>
    <label><input type="radio" name="q09a" value="c"> <code>running → completed</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="b" data-explanation="The running-to-paused-to-running path places paused on an indirect cycle, so its event is multi.">
    <legend>Why does <code>paused</code> have <code>multi</code> cardinality?</legend>
    <label><input type="radio" name="q09b" value="a"> It has no attributes</label>
    <label><input type="radio" name="q09b" value="b"> It is part of <code>running → paused → running</code></label>
    <label><input type="radio" name="q09b" value="c"> It can transition to <code>completed</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/fsm-self-loop/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/fsm-self-loop/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/fsm-self-loop/main.py
