# Where are the events?

An instrumented application chooses an exporter when it creates the Quent
context. The examples in this tutorial use the no-op exporter, so their events
are discarded. In this lesson, we'll show how to use an actually useful
exporter - the NDJSON exporter - to export events in a human-readible
self-describing form.

## Instrumentation API

These snippets show only how context construction changes when NDJSON support
is enabled for the generated instrumentation library. They assume the generated
module declaration and language-specific build wiring from the complete
examples.

```rust
use instrumentation::{Context, Minimal};
use quent_instrumentation::{
    ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};

let exporter = ExporterOptions::FileSystem(FileSystemExporterOptions::new(
    FileSystemFormat::Ndjson,
    "./events".into(),
));
let context = Context::<Minimal>::try_new(exporter)?;
```

```cpp
auto context = quent::Context::ndjson("./events");
```

```python
with quent.Context(quent.ExporterOptions.ndjson("./events")) as context:
    ...
```

## Output

If we choose the NDJSON exporter with `./events` as its output root like above,
for the `minimal` schema of the previous page, Quent creates a directory like
this:

```text
events/
└── 0199a1c2-3456-7890-abcd-ef0123456789/
    ├── model.qmi
    └── Task/
        └── 0199a1c2-4567-7890-abcd-ef0123456789.ndjson
```

The first UUID identifies the instrumentation context. This keeps events from
separate application runs or contexts isolated.

Each entity event stream gets its own directory, such as `Task`, containing one
or more UUID-named event files.

After the minimal model emits `started` and `ended` through the NDJSON exporter,
its `Task` file looks like this (with shortened example values):

```json
{"id":"0199...6789","timestamp":1789675200000000000,"data":"Started"}
{"id":"0199...6789","timestamp":1789675200000000123,"data":"Ended"}
```

Both events have the same `id` because they belong to the same `Task` entity
instance. `timestamp` is the event time in nanoseconds since the Unix epoch,
and `data` contains the generated event payload. Later lessons add attributes,
references, and other semantics to that payload.

Drop the context and any remaining handles before reading the files at the end
of a short-lived program. This gives the background exporter time to drain and
flush all queued events.

NDJSON is just one exporter option. Other options currently include MessagePack,
Postcard, a collector for distributed processes, a callback exporter for
inprocess consumption and tests, and we'll be adding a few more soon.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../../../ui/public/logo.svg}}
  </div>
  <div>
    <strong>Key point</strong>
    <p>The output root contains one directory per instrumentation context, with one subdirectory per entity event stream.</p>
  </div>
</div>

`model.qmi` records all sorts of information about the build of the application
and Quent used to produce these events. This makes it easy for additional tools
such as [`quent-open`](https://github.com/rapidsai/quent/tree/main/crates/open)
to always open event files using the same builds as the ones used to produce and
analyze the events.
