# Operating System

The Operating System semantic module identifies a Quent entity as a specific
operating-system process or thread. This allows entities to be correlated with
data from general-purpose event sources that identify processes and threads by
their native operating-system IDs rather than Quent UUIDs, such as captured
NVTX annotations or CPU stack samples.

Use this module when:

- An entity represents an entire process in a multi-process system, such as a
  worker process in a process pool.
- An entity represents a specific operating-system thread, such as a worker or
  main thread.
- The entity must be matched with captured NVTX annotations, CPU stack samples,
  or another source that identifies processes and threads by native IDs.

Do not use this module when:

- An error event only needs to report the process or thread in which the error
  occurred.
- A task, request, or connection event records which process or thread handled
  it, but the entity itself does not represent that process or thread.
- A process or thread ID is only a diagnostic field or metric dimension and
  does not define what the entity represents.

Use ordinary attributes for those values instead.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/operating-system/model.yaml}}
```

Using the `os` as an event field type adds Quent's standard operating-system
records to the schema and uses it as the field's type:

- `{ os: process }` adds the `quent::os::Process` record.
- `{ os: thread }` adds the `quent::os::Thread` record.

The generated event API requires values for these fields when the event is
emitted.

We use these standard records so event analysis tools know that the entity
itself represents a process or thread. Because every model uses the same record
names and fields, generated APIs and other tools can handle them consistently.

These fields can also be used as attributes of an FSM state. That state must not
be part of a cycle because states on a cycle may occur more than once.

Each identity record must be carried by a once-only event. A thread that does
not also represent a process must be scoped under its containing process. The
`process` reference on `Thread.started` establishes that relationship here.

## Instrumentation API

The event producer reports IDs from the native platform APIs. The generated API
then associates those IDs with the process and thread entities. These snippets
focus on the instrumentation API; the full sources linked below contain the
platform-specific code that reads the native IDs.

```rust
{{#include ../../../../../../crates/yaml/examples/operating-system/src/main.rs:9:33}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/operating-system/main.cpp:6:6}}

{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/operating-system/main.cpp:11:12}}

{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/operating-system/main.cpp:58:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/operating-system/main.py:4:27}}
```

## Platform considerations

Native IDs identify operating-system objects only within the scope and lifetime
defined by the platform. They are not universally unique identifiers.

On Linux, the process ID and the main thread's thread ID have the same numeric
value. macOS and Windows obtain process and thread IDs through separate native
APIs. Consumers must distinguish the two identities by their record types, not
by comparing their numeric values.

An entity can carry both records when it represents both a process and its main
thread. This example uses separate entities so the process-to-thread scope is
explicit.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>OS identity connects an entity to an external process or thread identity; it does not add IDs to every event.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="13">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="The OS identity lets tools correlate a Quent entity with external data keyed by its native process or thread ID.">
    <legend>Why should a model use an OS process or thread identity?</legend>
    <label><input type="radio" name="q13a" value="a"> To add a process ID to every event</label>
    <label><input type="radio" name="q13a" value="b"> To correlate an entity with external OS-scoped event streams</label>
    <label><input type="radio" name="q13a" value="c"> To replace the entity's Quent UUID</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="A thread-only entity must be transitively scoped under the entity representing its containing process.">
    <legend>How is a thread-only entity associated with its containing process?</legend>
    <label><input type="radio" name="q13b" value="a"> By requiring equal process and thread IDs</label>
    <label><input type="radio" name="q13b" value="b"> By using the process ID as its Quent UUID</label>
    <label><input type="radio" name="q13b" value="c"> By a tree-forming scope reference to the process entity</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/operating-system/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/operating-system/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/operating-system/main.py
