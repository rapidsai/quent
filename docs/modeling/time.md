# Time-related concepts

## Timestamp

Timestamps are 64-bit unsigned integers (`u64`) representing the amount of
nanoseconds passed since the Unix Epoch as defined in the
[POSIX](https://posix.opengroup.org/) standard (IEEE Std 1003.1-2024).

> Rationale: The choice of nanoseconds represented as `u64` values allows
> timestamps to extend approximately $`584.6`$ average Gregorian years past the
> Unix Epoch.

## Span

A Span is a half-open interval `[start, end)` over two [Timestamps][timestamp]:

- `start: Timestamp`: the beginning (inclusive)
- `end: Timestamp`: the end (exclusive)

The `end` [Timestamp][timestamp] must be equal to or greater than the
`start` [Timestamp][timestamp].

## Duration

A Duration is the absolute difference between two [Timestamps][timestamp] (
`u64`). A Duration always represents how much time has elapsed on a wall-clock.

[timestamp]: #timestamp
