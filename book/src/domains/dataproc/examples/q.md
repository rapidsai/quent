# Q: a contrived engine to examplify the modeling technique

> 🚧 WORK IN PROGRESS 🚧

This section will describe an example of a model of a contrived distributed
query engine called "Q".

## High-level description

Q can have multiple [Workers][worker]. Q defines two
[Plan][plan] levels: a "logical" [Plan][plan] and a "physical" [Plan][plan].
Each [Worker][worker] has an instance of a "physical" [Plan][plan] with the
exact same topology.

Q is very simple. After performing a topological sort of the physical
[Plan][plan], its scheduling thread visits every physical [Operator][operator]
of the [Plan][plan] and enqueues a single Task to a Thread Pool
that runs on a Thread until all work of that single Operator is completed.

While the Task is running on the Thread, it can load a RecordBatch from the
Filesystem, which represents a [Worker][worker]-local partition of a table, and
spill any of its input to the Filesystem if it cannot get an Allocation for both
its inputs and worst-case sized outputs, while it keeps trying to
[Allocate][allocation] [Memory][memory] to write the output of some
[Computation][computation]. As such, Tasks running in Q can make room for other
concurrent Tasks, but if the sizes of their input and output RecordBatches
together would exceed total memory capacity, it will simply fail the query. It
may be best to not perform full outer joins on Q.

While the Task is running on the Thread, it can also split up a RecordBatch and
send it to [Memory][memory] of another [Worker][worker].

## Entities

### Resources

#### Worker-scoped

- Filesystem: [Memory][memory]
- MainMemory: [Memory][memory]
- FsToMem: [Channel][channel] between Filesystem and MainMemory
- MemToFs: [Channel][channel] between MainMemory and Filesystem
- Task Thread: [Processor][processor]
- Thread Pool: [Resource Group][resource-group] of Task Threads

#### Engine-scoped

- Link: [Channel][channel] between the MainMemory of two different Workers
- Network: [Resource Group][resource-group] of Links

### Control flow

#### Entities required by the domain-specific model

- [Engine][engine]
- [Query Group][query-group]
- [Worker][worker]
- [Query][query]
- [Plan][plan]
- [Operator][operator]

### Application-specific entities

- RecordBatch (FSM)
  - Relates to:
    - Operator
  - The `idle` state claims an [Allocation][allocation] in either Filesystem or
    MainMemory.
  - State transitions:

    ```text
    ⊙             -> initializing
    initializing  -> idle
    idle          -> moving
    moving        -> idle
    idle          -> finalizing
    finalizing    -> ⊗
    ```

- Task (FSM)
  - Relates to:
    - Operator
  - All states except `initializing`, `queueing`, and `finalizing` claim a
    [Computation][computation] of one and the same Task Thread.
  - The `sending` state claims a [Transfer][transfer] of a Link
  - The `loading` state claims a [Transfer][transfer] of a FsToMem
  - The `spilling` state claims a [Transfer][transfer] of a MemToFs
  - State transitions:

    ```text
    ⊙                 -> initializing
    initializing      -> queueing
    queueing          -> allocating memory
    allocating memory -> allocating storage -> spilling -> allocating memory
    allocating memory -> loading -> computing
    allocating memory -> computing
    computing         -> sending
    sending           -> finalizing
    sending           -> sending
    computing         -> finalizing
    finalizing        -> ⊗
    ```

### Model relations

The lowest-level Entities of the model of Q are the Task and the RecordBatch.
A consistent model is able to relate any Entity all the way back to an Engine.

- For Task and RecordBatch, this can be done as follows:

```text
Task/RecordBatch -> Operator -> Plan (physical) -> Plan (logical) -> Query -> Query Group -> Engine
```

Note the above is not some FSM definition, but merely describes how construct
are related through their [Attributes][attributes].

A consistent model also ensures all defined [Resources][resource] have a
[Use][use] somewhere, which in the case of the model of Q:

```text
Task (computing, allocating memory/storage, loading, sending) -> Computation -> Task Thread -> Thread Pool -> Worker -> Engine
Task (loading) -> Transfer -> FilesystemIO -> Filesystem -> Worker -> Engine
Task (sending) -> Transfer -> Link -> Network -> Engine
RecordBatch (idle, moving) -> Allocation -> Memory / Storage -> Worker -> Engine
```

Because all Entities in the model of Q can be related back to the
Engine, a relation graph virtually exists that connects all Engine concepts.

### Notes on Analysis

The model of Q, when combined with telemetry capturing events that
provide data according to the model of Q, will allow answering many questions or
provide the means to visualize performance. Here are some examples provided in
the order in which an analyst may traverse through an interactive performance
analysis tool.

- Given an engine id, list all query groups named "tpc-h benchmark"
- Given the query grouyp id, list all queries named "21"

- Given a query id, show a DAG of the logical and physical [Plan][plan]
- In the DAG of the logical [Plan][plan], show the number of input and output
  rows for each [Port][port] of an [Operator][operator].
- In the DAG of the logical [Plan][plan], show the average throughput of a Task
  sending data through the Network.
- In the DAG of the logical [Plan][plan], color the [Operators][operator] with
  colors from a colorblindness-friendly heatmap that corresponds to the number
  of bytes transfered trough the Network.
- In the DAG of the physical [Plan][plan], color the [Operators][operator] with
  colors from a colorblindness-friendly heatmap that corresponds to total amount
  of time spent in a Task Thread.
- In the DAG of the physical [Plan][plan], show the maximum number of bytes
  claimed Memory Allocations.

- Given a query id, show a timeline of Tasks running on Thread Pool Threads,
  giving each Task state a unique colorblindness-friendly color.
- Given a query id, show a timeline with a Memory usage graph based on
  Allocations.
- etc.

Herein lies the power of a generic model for query engines - rather than N
engines implementing N performance analysis tools that roughly do the same
thing, there can be a much smaller set of performance analysis tools.

[allocation]: ../../../modeling/common/memory.md#allocation
[attributes]: ../../../modeling/attributes.md
[channel]: ../../../modeling/common/channel.md
[computation]: ../../../modeling/common/processor.md#computation
[engine]: ../README.md#engine
[memory]: ../../../modeling/common/memory.md
[operator]: ../README.md#operator
[plan]: ../README.md#plan
[port]: ../README.md#port
[processor]: ../../../modeling/common/processor.md
[query]: ../README.md#query
[query-group]: ../README.md#query-group
[resource]: ../../../modeling/resource.md
[resource-group]: ../../../modeling/resource_group.md
[transfer]: ../../../modeling/common/channel.md#transfer
[use]: ../../../modeling/resource.md#use
[worker]: ../README.md#worker
