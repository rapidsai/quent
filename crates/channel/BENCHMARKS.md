# Channel benchmarks

Run `pixi run cargo bench -p quent-channel --bench channel --locked` from the
repository root. The benchmark writes CSV to standard output; Cargo's build
messages go to standard error.

Each scenario sends 100,000 `usize` values. `steady` alternates one push and one
receive on the same thread after setup and measures the push only. `burst`
measures pushes while the receiver is idle, then verifies FIFO delivery.
`parallel-4p` runs four producer threads against one active collector and
includes their first registration. `registration` measures the first send on
1,000 fresh thread-local channels. Push samples use `Instant`, so clock overhead
is included. `total_ms` and `events_per_second` include the scenario loop and,
where applicable, concurrent collection. Allocation counts cover the same
interval as `total_ms`; they include benchmark setup inside that interval, such
as worker sample buffers in `parallel-4p`.

The comparison includes a preallocated fixed-size `rtrb` ring, the thread-local
registered channel, and unbounded channels from the standard library, Crossbeam,
Flume, Kanal, and Tokio. The fixed `rtrb` burst has capacity for the entire run,
so it measures a bounded preallocation baseline rather than unbounded growth.
The `steady` scenario does not measure inter-core communication; use
`parallel-4p` for a concurrent collector. Results depend on allocator, CPU
placement, system load, and build profile. Run several times on the target
machine before choosing a configuration.
