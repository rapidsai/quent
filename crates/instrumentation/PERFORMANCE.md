# Performance

> **TL;DR:**
>
> **What is the Quent configuration with the lowest possible latency?**
>
> Enable the `channel-per-thread` and `clock-quanta` features on
> `quent-instrumentation` at your own risk (explained below).
>
> **What else can I do to reduce overhead?**
>
> Consider how long your program takes to calculate attribute values. This might
> add overhead to the program that wasn't there before you instrumented it,
> since you might not have been doing those calculations before. Also, don't
> spam too many events from your critical path.

Quent is very fast by default, but it can be made even faster by leveraging
certain approaches enabled by Cargo features. The trade-offs are explained here.

The default features are chosen such that the highest level of guarantees about
properly measuring time and ensuring all events are exported are provided.

Only if you need more performance, it might be interesting to read on. If your
program behaves well enough, these Cargo features can help easily gain some
performance without changing anything else.

## What happens when I emit an event through the instrumentation API?

By default, Quent uses a concurrent multi-producer, single-consumer channel from
[Tokio](https://tokio.rs/) to move events created by instrumentation calls into
a task running on a background thread that deals with exporting events.

## What is the overhead of Quent?

The default MPSC channel used keeps the latency of instrumentation calls low,
since it moves most of the work involved in exporting to a background thread. In
isolation, instrumentation calls do not slow down the instrumented program much.
Depending on the system, the event pattern and especially on how many threads
are emitting events simultaneously, the latency is typically in the order of
tens to hundreds of nanoseconds per call. You can measure this with
`quent-bench` for your system.

However, Quent is instrumentation-based, which means you decide where you call
instrumentation events in your code, how many attributes you put in your events,
and how much work is involved in obtaining those attributes.

Therefore, there is no way to answer the question "What is the overhead of
Quent?" without asking yourself these questions first:

1. **How many events are you going to produce in your critical path?**

The minimum amount of overhead added is this number multiplied by the latency of
bare instrumentation calls. But if many threads emit events at the same time,
there can be some contention on writing to the channel and the latency can
significantly increase.

2. **How long does it take you to obtain the attribute values of those events in
   your critical path?**

Add this latency too.

3. **How many events are you going to create per second (throughput)?**

This is tricky to do back-of-the-envelope math for to arrive at some estimation.
The overhead depends on many factors related to how exporters work.
Nevertheless, it is important to understand that if you produce an excessive
amount of events, then the background threads dealing with exporting will start
eating up a lot of your system's resources. So even if the latency of
instrumentation calls remains low, it might slow down the entire program as the
background threads will eat up a lot of resources such as memory, CPU cycles,
and I/O.

Once you have a clear answer to these questions, and you have carefully
considered what you are doing to instrument your program, you may still not be
satisfied with the overhead that Quent adds. In this case, read on.

## Can I reduce the latency of instrumentation calls?

If many threads create events at the same time, they can slow each other down
when they write to the default channel. You can enable the `channel-per-thread`
feature to give each thread its own queue which consists of multiple segments of
ring buffers. This can significantly reduce the time spent in an instrumentation
call, especially when many threads are active, since they don't have to contend
to write to the same channel.

It also means that each thread needs memory for its queue, including segments
kept for reuse after a burst, so if that is not an objection, you can consider
using this channel.

Quent also reads a clock to timestamp every event. By default, it uses Rust's
`std::time::Instant`. You can enable `clock-quanta` to use Quanta's clock
instead, which may make timestamp reads faster on your system.

## What happens when I stop instrumentation?

Quent continues forwarding an entity's events until the last object that owns
its instrumentation is dropped. Dropping its model `Context` may not start
shutdown if cloned observers or entity handles are still alive.

The guarantees below say what has happened by the time shutdown returns. When
an event reaches the exporter, Quent has handed it to the code that writes
(or sends) it out. It does not mean that a file or remote collector has saved
it. Also, instrumentation calls do not tell you whether their events were
accepted, so a call can return normally even if its event is not exported.

Quent currently provides two levels of shutdown guarantee:

| Level | Guaranteed to reach the exporter before shutdown returns | Worst-case behavior |
| ----- | ------------------------------------------------------- | ------------------- |
| 1 | Events from calls completed before shutdown begins. | Calls overlapping or following shutdown may enqueue events that never reach the exporter. |
| 2 | Every event accepted by the channel. | Sends racing with channel closure may be rejected. Sends after closure are always rejected. |

The available channels provide these levels:

| Channel               | Level | What this means for you                                                                           |
| --------------------- | ----- | ------------------------------------------------------------------------------------------------- |
| Default Tokio channel | 2     | An event is either rejected by the channel or forwarded to the exporter before shutdown returns.  |
| `channel-per-thread`  | 1     | A thread may still accept events after shutdown returns. Those events may not reach the exporter. |

Keep the default Tokio channel if sends attempted after instrumentation shutdown
must be rejected rather than queued without an active exporter.

If you stop and join all threads that can still create events before dropping
the last owner of the instrumentation, which is good practice in general, Level
1 guarantees that every event from those completed calls reaches the exporter
before shutdown returns. Both channels log exporter failures instead of
reporting them to the caller.

With `channel-per-thread`, shutdown drains at most the buffered event count
observed during closure plus one segment's capacity per producer queue.
It stops earlier if no events are available. Later sends do not extend this
budget, so producers cannot keep shutdown draining indefinitely by refilling
their queues.

To use `channel-per-thread`, add it to the feature list of your existing
`quent-instrumentation` dependency. Cargo features apply to the whole build. If
any dependency enables this feature, all uses of `quent-instrumentation` use the
new channel. You cannot choose a different channel for each `Context`.

## In what order will my events be exported?

This totally depends on the exporter. The simple "filesystem" exporters
`ndjson`, `postcard`, and `messagepack` simply export in the order at which
events arrive on the receiving side of the event channel.

With `channel-per-thread`, events from one thread stay in order except during
thread-local destruction, when events may reach the exporter before that
thread's earlier events. Events from different threads may reach the exporter
in a different order from when they were created.

Since every event has a timestamp (and for finite-state-machines also a sequence
number), this usually doesn't matter, as in analysis you'll order them by
timestamp anyway. But if two threads emit events through separate handles for
the same entity, coordinate those threads if event order matters,
because even the most "correct" timestamping mechanism that Quent could possibly
use today has caveats (also see the `quent-time` crate, this is a whole
rabbithole on its own). You can only make this potentially worse through
different / faster clock configurations. If you want to be sure, do not use
`clock-quanta`.

The background exporter checks these queues every 1 ms when it has no events to
process. This saves the work of waking it for every event, but an event may wait
for the next check before it is exported.
