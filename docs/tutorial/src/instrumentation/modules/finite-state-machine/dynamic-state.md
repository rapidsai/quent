# Dynamic state

Typestate handles are the preferred FSM API because they make invalid
transitions impossible to express at compile-time. Sometimes, however, a
function can finish in one of several states selected at runtime.

Consider a function that prepares the `Job` from the
[basic lifecycle](basic-lifecycle.md). It either loads input or restores a
checkpoint:

```text
queued ─┬─> loading_input ───────────┐
        └─> restoring_checkpoint ────┴─> running ─> completed
```

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/fsm-dynamic-state/model.yaml}}
```

The two branches return different Rust types:

```rust
fn prepare_job(
    job: FsmHandle<Job, job_state::Queued>,
    restore: bool,
) -> /* no single typestate handle works here */ {
    if restore {
        job.restoring_checkpoint() // FsmHandle<Job, RestoringCheckpoint>
    } else {
        job.loading_input()        // FsmHandle<Job, LoadingInput>
    }
}
```

An enum containing both handle types could represent this result, but it would
be specific to this function's two possible outcomes. Quent knows the FSM
topology, but it cannot know in advance which subsets and combinations of
states application control flow will need to return. Generating enums for every
possible combination would cause the API surface to grow rapidly, while asking
applications to define them creates a different wrapper type for each such
boundary. Consumers would also have to match each enum before doing common
work.

## Instrumentation API

Calling `into_dynamic()` consumes any typestate handle and returns the common
`DynamicFsmHandle<Job>` type. The handle stores its current state and exposes
all transitions declared by the FSM. Each example below returns the dynamic
handle from the same runtime branch:

```rust
{{#include ../../../../../../crates/yaml/examples/fsm-dynamic-state/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/fsm-dynamic-state/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/fsm-dynamic-state/main.py:4:}}
```

The tradeoff is when transition correctness is checked. `FsmHandle` lets the
compiler (if any) check it through the Rust or C++ type system, or through
typechecks for Python type stubs. `DynamicFsmHandle` checks it when the
transition method is called. An invalid transition returns `FsmTransitionError`
in Rust, throws from the C++ method, or raises `InvalidFsmTransitionError` in
Python. A failed transition does not emit an event or change the stored state.

Converting a typestate handle preserves its current state and entity ID. In
Python, the generated type stubs still distinguish each typestate handle, while
the dynamic handle gives type checkers a common return type for both branches.

When the current state becomes known again, convert back to a typestate handle.
Rust uses `try_into::<job_state::Running>()`, C++ uses
`try_into<quent::job_state::Running>()`, and Python exposes the state-specific
`try_into_running()` method. A successful conversion consumes the dynamic
handle. A mismatch does not: Rust returns it in `FsmStateMismatch`, C++ returns
an empty `std::optional` without moving from the handle, and Python raises
`InvalidFsmStateError` without consuming it.

Instrumentation supports up to 255 declared states in one FSM. Dynamic handles
use a compact state index, with one additional value for a handle that has not
entered its initial state. Code generation rejects an FSM with 256 or more
states.

## When to use it

Use `DynamicFsmHandle` when an API boundary must represent multiple possible
current states, such as a function that resumes persisted work, performs an
optional preparation phase, or reconstructs state from runtime input.

Keep `FsmHandle` when the current state is known to the caller. It provides the
stronger guarantee, offers only valid transitions in editor completion, and
does not require handling runtime transition errors.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>Convert to <code>DynamicFsmHandle</code> at the boundary where control flow prevents one typestate return type, not earlier.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="fsm-dynamic">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="A dynamic-state handle provides one return type when runtime control flow can leave the FSM in different states.">
    <legend>When should a function return a <code>DynamicFsmHandle</code>?</legend>
    <label><input type="radio" name="q-fsm-dynamic-a" value="a"> Whenever the current state is known to the caller</label>
    <label><input type="radio" name="q-fsm-dynamic-a" value="b"> When runtime control flow can return the FSM in different states</label>
    <label><input type="radio" name="q-fsm-dynamic-a" value="c"> Only when the FSM has a self-loop</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="Dynamic-state transitions are checked when called; a rejected transition does not emit an event or change the stored state.">
    <legend>What happens when a dynamic-state handle attempts an invalid transition?</legend>
    <label><input type="radio" name="q-fsm-dynamic-b" value="a"> The transition is emitted and the state remains unchanged</label>
    <label><input type="radio" name="q-fsm-dynamic-b" value="b"> The handle silently moves to the target state</label>
    <label><input type="radio" name="q-fsm-dynamic-b" value="c"> The call reports an error without emitting an event or changing state</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/fsm-dynamic-state/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/fsm-dynamic-state/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/fsm-dynamic-state/main.py
