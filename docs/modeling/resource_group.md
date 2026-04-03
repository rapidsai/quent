# Resource Group

A Resource Group is an [Entity][entity] that represents a hierarchical grouping
over a set of [Resources][resource] and other Resource Groups.

Must have:

- `instance_name: string`: the name of the instance of this Resource Group.

May have:

- `parent_group_id: uuid`: the id of the parent resource group, if any.

Exactly one Resource Group must exist for which `parent_group_id` is null.

## Notes

Resource Groups together with [Resource][resource] types are useful to express
[Resource][resource] hierarchies as trees in which [Resource][resource]
[Usages][usage] may be aggregated.

For example, consider an application that divides a workload up into two parts:
A and B. For simplicity, assume only these parts allocate memory. They both
ultimately allocate from some root allocator, through two separate memory pool
instances individually dedicated to A and B.

One way a developer can model this by first declaring a memory
[Resource][resource] with a `bytes` [Capacity][capacity] with
[Resource][resource] type name `LeafPool`. By grouping the `LeafPool` instances
into a Resource Group named `RootPool`, a telemetry analysis tool can, without
necessarily requiring application-specific code, respond to questions such as
"give me a timeline of the `sum` of `bytes` allocated by all `LeafPool`s under
the `RootPool` group", in order to get an overview of the memory utilization in
the system over time. This requires [Resource][resource] type names as a way of
telling the analysis tool: the developer finds it sensible to allow aggregating
over the `byte` [Capacity][capacity] of all instances of this
[Resource][resource] type.

In the case of different underlying types of memory, e.g. a host and device
memory in a GPU-accelerator scenario, it typically would not make sense to
aggregate `byte` [Capacities][capacity] of separate pools dedicated to allocate
in either memory. Thus, different types of `LeafPool` [Resources][resource]
should be modeled, e.g. a `HostLeafPool` and a `GPULeafPool`, so the total
number of `bytes` are aggregated and visualized separately.

### Domain entities as Resource Groups

In domain-specific models (e.g. the query engine domain), domain entities
themselves may implement the Resource Group interface. For example, in the query
engine domain, Engine, Query, Plan, Operator, and Port all act as Resource
Groups, forming a hierarchy:

```text
Engine (root, parent_group_id=None)
├── Worker (parent=engine)
├── QueryGroup (parent=engine)
│   └── Query (parent=query_group)
│       └── Plan (parent=query or worker)
│           └── Operator (parent=plan)
│               └── Port (parent=operator)
└── Application Resources (memory, processor, channels)
```

This allows resource usages to be aggregated at any level of the hierarchy.

[capacity]: ./resource.md#capacity
[entity]: ./entity.md
[resource]: ./resource.md
[usage]: ./resource.md#usage
