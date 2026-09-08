# Instrumentation runtime microbenchmarks

Measures the caller-side cost of `EventSender::emit` with each provided
exporter, plus a `noop` baseline.

## Run

```sh
cargo bench -p quent-instrumentation --bench event_emit
```

Report: `target/criterion/report/index.html`.

## Run with profiling

```sh
QUENT_BENCH_PROFILE_TIME=10 cargo bench -p quent-instrumentation --bench event_emit
```

Flamegraphs: `target/criterion/emit/<variant>/profile/flamegraph.svg`.

Setting `QUENT_BENCH_PROFILE_TIME` (seconds, per variant) switches criterion
from measurement to profile mode and writes one flamegraph SVG per variant.
`QUENT_BENCH_PROFILE_HZ` overrides the SIGPROF sampling rate (default 4999 Hz):

```sh
QUENT_BENCH_PROFILE_TIME=10 QUENT_BENCH_PROFILE_HZ=9999 cargo bench -p quent-instrumentation --bench event_emit
```

## Clear results

```sh
rm -rf target/criterion
```
