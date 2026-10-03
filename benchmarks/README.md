# `quent-bench`

`quent-bench` measures instrumentation latency.

## Run

From the repository root:

```sh
pixi run cargo run --release -p quent-bench -- --frameworks quent --empty-loop --threads 1,2,4 --num-batches 1000 --batch-size 20 --num-warmup-batches 10 --batch-pause-us 10
```

`quent-bench` builds implementations when needed and runs each case in a
separate process. Cases run sequentially so their global resources do not
overlap. It prints a summary table and writes a JSON report.

| Argument               | Default          | Meaning                                                                                 |
| ---------------------- | ---------------- | --------------------------------------------------------------------------------------- |
| `--frameworks`         | `quent`          | Frameworks to measure, as a comma-separated list. Other frameworks are WIP.             |
| `--empty-loop`         | Off              | Add an empty-loop measurement for each selected language.                              |
| `--event-shape`        | All shapes below | Event payloads to measure, as a comma-separated list.                                   |
| `--threads`            | `1`              | Concurrent caller threads, as comma-separated positive counts.                          |
| `--num-batches`        | `1000`           | Measured batches per thread.                                                            |
| `--batch-size`         | `20`             | Calls or empty-loop iterations per thread in each batch.                               |
| `--num-warmup-batches` | `10`             | Untimed batches before measurement.                                                     |
| `--batch-pause-us`     | `10`             | Minimum per-thread busy wait between batches, in microseconds. Set to `0` for no pause. |
| `--no-preflight-call`  | Off              | Skip one untimed call or empty-loop iteration per thread.                              |
| `--output PATH`        | Generated file   | JSON report path.                                                                       |

The default report path is
`benchmarks/results/results-YYYY-MM-DD-HH-MM-SS-pidN.json`, using
local time.

### Framework-specific options

#### Quent

| Argument           | Default                        | Meaning                                       |
| ------------------ | ------------------------------ | --------------------------------------------- |
| `--quent-channel`  | `tokio,spsc`                   | Select channel variants from a comma-separated list. |
| `--quent-exporter` | `noop,ndjson,msgpack,postcard` | Select exporters from a comma-separated list. |

`tokio` uses Quent's default unbounded mpsc channel and appears as `quent` in
the report. `spsc` enables the `channel-spsc` Cargo feature and appears as
`quent-spsc`. Each variant is built separately and runs in a separate process.
Their different shutdown guarantees are documented in
[`crates/instrumentation/PERFORMANCE.md`](../crates/instrumentation/PERFORMANCE.md).

Available `--quent-exporter` values:

| Value      | Behavior                                                       |
| ---------- | -------------------------------------------------------------- |
| `noop`     | Discards events.                                               |
| `ndjson`   | Writes NDJSON to temporary files.                              |
| `msgpack`  | Writes length-prefixed MessagePack records to temporary files. |
| `postcard` | Writes length-prefixed Postcard records to temporary files.    |

Quent creates one context and observer pipeline per case, with one entity
handle per thread. File exporters serialize and write during measurement.
After draining, the benchmark counts exported records. The no-op path may drop
prepared strings during timing.

### Event shapes

Each call emits one event with the selected explicit attributes. Quent's
`instr_call` also contains a timestamp and entity UUID, including for `empty`.
Payloads are prepared outside timing.

| Shape          | Explicit attributes                                                       |
| -------------- | ------------------------------------------------------------------------- |
| `empty`        | None                                                                      |
| `u8`           | `value: u8`                                                               |
| `u64`          | `value: u64`                                                              |
| `short-string` | `value: string` (8-16 bytes)                                              |
| `long-string`  | `value: string` (128-256 bytes)                                           |
| `all`          | `small: u8`, `large: u64`, `short: string`, `long: string` (same lengths) |

## Measurement

The latency benchmark groups instrumentation calls into batches and reports an
average per call for each batch. Instrumentation calls can cost about as much
as, or less than, reading a monotonic clock. Timing each call separately would
add clock overhead comparable to the call itself, so each pair of clock reads is
spread across `--batch-size` calls.

The benchmark simulates time spent on other CPU work between instrumentation
bursts by having each thread busy-wait for `--batch-pause-us`. A pause of `0`
runs batches without a requested delay.

The measurement proceeds as follows across all implementations / languages:

1. Each thread makes one untimed preflight call by default so any potential lazy
   first-call setup overhead stays outside measurements.
2. Each thread prepares all batch payloads before the first batch barrier.
   Threads meet at a barrier before every batch.
3. Each thread runs `--num-warmup-batches` batches without recording time, to
   exercise the call path before measurement.
4. For each measured batch, every thread reads a monotonic clock, makes
   `--batch-size` calls, then reads the clock again.
5. Threads meet at another barrier, then busy-wait for at least
   `--batch-pause-us` before the next batch when the pause is nonzero.

The benchmark reports `sum(thread_batch_elapsed_ns) / (threads * num_batches *
batch_size)` in nanoseconds per call, plus sample standard deviation and
nearest-rank p50/p95/p99 of batch averages.

The report also gives total calls and calls without a retained event. These
counts include warmup and optional preflight calls. No-op exporters discard all
calls by design. The empty loop has no event, so its discarded count is unset.

With `--empty-loop`, `quent-bench` times empty-loop iterations using the same
batch schedule. This gives a rough indication of loop and timing overhead.
Compilers may optimize an instrumentation loop differently, so the empty-loop
result is not subtracted from event measurements.

### Clock sources

- **Rust:** `std::time::Instant::now()` starts each measured batch and
  `Instant::elapsed()` ends it. On Linux, `Instant` currently uses
  `clock_gettime(CLOCK_MONOTONIC)`. On macOS, it uses
  `clock_gettime(CLOCK_UPTIME_RAW)`. See the
  [Rust documentation](https://doc.rust-lang.org/std/time/struct.Instant.html).

## Why `quent-bench`?

`quent-bench` applies the same measurement protocol across languages. Matching
that protocol with separate frameworks such as Criterion, Google Benchmark, and
pyperf would be harder.
