# Directed Acyclic Graph

The Directed Acyclic Graph (DAG) module allows entities to represent a directed
acyclic graph. One type is the DAG, one or more types are vertices, and directed
edge types connect vertices. A vertex points to its DAG with `in`. An edge
points to its DAG and declares its source and target vertex types.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/dag/model.yaml}}
```

The `plan`, `source`, and `target` attributes are typed entity references.
Each vertex and edge must point to its DAG exactly once, using one membership
field in a once-event. An edge must declare its DAG, source, and target in that
same once-event. Its source and target types must be vertices of the same DAG
type.

The rules above describe the graph's types, not its individual instances. That
instance-level work belongs to analysis, which reconstructs the event data and
checks that edges belong to one DAG instance and form no cycles.

## Instrumentation API

Each generated API accepts typed entity references. DAGs use the usual event
methods; the DAG declarations add these graph rules to the schema.

```rust
{{#include ../../../../../../crates/yaml/examples/dag/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/dag/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/dag/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>DAG topology is recorded with typed references on ordinary events.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="dag">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="The source field targets Operator, so the generated parameter accepts an Operator reference.">
    <legend>Which reference type does <code>PlanEdge.connected</code> accept for <code>source</code>?</legend>
    <label><input type="radio" name="qDagA" value="a"> <code>EntityRef&lt;Plan&gt;</code></label>
    <label><input type="radio" name="qDagA" value="b"> <code>EntityRef&lt;Operator&gt;</code></label>
    <label><input type="radio" name="qDagA" value="c"> Any entity reference</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="A cycle can only be checked after the emitted references are reconstructed.">
    <legend>When can Quent determine whether the emitted plan contains a cycle?</legend>
    <label><input type="radio" name="qDagB" value="a"> While parsing the YAML alone</label>
    <label><input type="radio" name="qDagB" value="b"> While compiling the generated method call</label>
    <label><input type="radio" name="qDagB" value="c"> After reconstructing the emitted event data</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/dag/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/dag/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/dag/main.py

## Future work

The supported representation records vertices and edges as separate entities.
Another possible representation is to carry the full topology in one event on
the DAG entity; that representation is not currently supported by this module.
