# Log Sink

The Log Sink semantic module models an entity as a logging destination. Each
declared level becomes a repeatable event with an implicit `message: string`
attribute. The level's position in the declaration is its zero-based severity
rank.

## YAML model

```yaml
{{#include ../../../../../../crates/yaml/examples/log-sink/model.yaml}}
```

`AppLog` is the static logging scope, while each `AppLog` handle identifies one
runtime sink instance. The `target` attribute preserves a facade-provided
logical category. The source attributes preserve the file, line, and module of
the call site.

`thread_name` is common to every level. `category` appears only on `warning`,
and `error_code` appears only on `error`. Every generated level event also
contains `message` and is repeatable.

## Instrumentation API

The generated API exposes one method per declared level. This example calls it
directly; a logging-facade adapter can supply the same arguments later without
changing the schema contract.

```rust
{{#include ../../../../../../crates/yaml/examples/log-sink/src/main.rs}}
```

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>The entity identifies the logging scope, and the event identifies the level.</p>
  </div>
</div>

<section class="check-yourself" data-lesson="log-sink">
  <h2>Check yourself</h2>
  <fieldset data-answer="b" data-explanation="Each declared level becomes a repeatable event with an implicit message attribute.">
    <legend>How does a log level appear in the generated schema?</legend>
    <label><input type="radio" name="qLogA" value="a"> As a runtime <code>log_level</code> attribute</label>
    <label><input type="radio" name="qLogA" value="b"> As a repeatable event named after the level</label>
    <label><input type="radio" name="qLogA" value="c"> As a separate entity</label>
    <p class="question-feedback"></p>
  </fieldset>
  <fieldset data-answer="c" data-explanation="A separate log entity defines a separate static scope; target and module remain optional event data.">
    <legend>How do you define a separate static logging scope?</legend>
    <label><input type="radio" name="qLogB" value="a"> Emit a different <code>target</code></label>
    <label><input type="radio" name="qLogB" value="b"> Enable <code>source.module</code></label>
    <label><input type="radio" name="qLogB" value="c"> Declare another entity with a <code>log:</code> block</label>
    <p class="question-feedback"></p>
  </fieldset>
  <button type="button" class="check-answers">Check answers</button>
  <p class="quiz-result" aria-live="polite"></p>
</section>
