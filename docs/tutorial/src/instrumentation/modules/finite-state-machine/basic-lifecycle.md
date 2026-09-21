# Basic lifecycle

An FSM puts lifecycle topology in the model. It declares an initial state,
allowed transitions, and a reachable final state.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/finite-state-machine/model.yaml}}
```

The parser rejects missing initial states, unreachable states, invalid targets,
and FSMs without a reachable final state. It also derives event cardinality from
the topology.

## Instrumentation API

Entering a state emits its generated event. The generated API represents the
current FSM state in the handle's type. Each transition consumes that handle
and returns a handle for the target state. Only transitions allowed from the
current state are available to the compiler or type checker. This pattern is
called typestate.

The Rust and C++ examples show the state-specific handle types directly. The
generated Python type stubs expose the same transition constraints to type
checkers and editors.

```rust
{{#include ../../../../../../crates/yaml/examples/finite-state-machine/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/finite-state-machine/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/finite-state-machine/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>The generated handle exposes only the transitions allowed from its current state.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="08">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="The FSM validator checks the initial state, reachability, transition targets, and a final path.">
    <legend>Which property does the parser validate for this FSM?</legend>
    <label><input type="radio" name="q08a" value="a"> Exporter throughput</label>
    <label><input type="radio" name="q08a" value="b"> State reachability and a final path</label>
    <label><input type="radio" name="q08a" value="c"> Runtime thread ownership</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="b" data-explanation="Both loading_input and restoring_checkpoint declare running in their to lists.">
    <legend>Which states can directly precede <code>running</code>?</legend>
    <label><input type="radio" name="q08b" value="a"> Only <code>queued</code></label>
    <label><input type="radio" name="q08b" value="b"> <code>loading_input</code> or <code>restoring_checkpoint</code></label>
    <label><input type="radio" name="q08b" value="c"> Only <code>loading_input</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/finite-state-machine/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/finite-state-machine/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/finite-state-machine/main.py
