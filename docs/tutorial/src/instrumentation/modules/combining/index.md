# Combining modules

Module constraints can be used together in one model. The job workload combines
a finite-state lifecycle with bounded resource usage.

This example combines an FSM, event attributes, and measured resource usage. A
worker publishes its thread limit. A job records how many threads it requests
and how many it occupies while running.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/job-workload/model.yaml}}
```

## Instrumentation API

The generated API distinguishes the worker's `WorkerBounds` from the job's
`WorkerUsage`. No event names or payload keys are assembled at runtime.

```rust
{{#include ../../../../../../crates/yaml/examples/job-workload/src/main.rs:9:}}
```

```cpp
{{#include ../../../../../../experimental/vibe/codegen/cpp/example/tutorial/job-workload/main.cpp:6:}}
```

```python
{{#include ../../../../../../experimental/vibe/codegen/python/example/tutorial/job-workload/main.py:4:}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>The job records its requested thread count and its usage of a bounded worker resource.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="13">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="WorkerBounds publishes the worker's available thread capacity.">
    <legend>What does <code>WorkerBounds { threads: 16 }</code> represent?</legend>
    <label><input type="radio" name="q13a" value="a"> Threads requested by one job</label>
    <label><input type="radio" name="q13a" value="b"> Threads available on the worker</label>
    <label><input type="radio" name="q13a" value="c"> Jobs completed by the worker</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="running accepts the worker reference carrying the job's WorkerUsage claim.">
    <legend>Which call records where the job runs and how many threads it occupies?</legend>
    <label><input type="radio" name="q13b" value="a"> <code>worker.ready(...)</code></label>
    <label><input type="radio" name="q13b" value="b"> <code>job.queued(...)</code></label>
    <label><input type="radio" name="q13b" value="c"> <code>job.running(...)</code></label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>

## Full code

- [Rust source][rust-source]
- [C++ source][cpp-source]
- [Python source][python-source]

[rust-source]: https://github.com/rapidsai/quent/blob/main/crates/yaml/examples/job-workload/src/main.rs
[cpp-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/cpp/example/tutorial/job-workload/main.cpp
[python-source]: https://github.com/rapidsai/quent/blob/main/experimental/vibe/codegen/python/example/tutorial/job-workload/main.py
