# Quent Schema Crate Architecture

A guided tour of the five foundational crates — `quent-schema`,
`quent-constraints`, `quent-fsm`, `quent-ref-target`, and `quent-ref-tree` —
covering their design, how they interplay at runtime, and how to wire them
together using a real-world Kubernetes platform example.

> **Companion reading.** This document explains the full five-crate system.
> For a deep-dive into `quent-ref-tree` specifically — its algorithm, internal
> graph, and error taxonomy — see
> [`ref-tree-explained.md`](./ref-tree-explained.md).

---

## 1. The five crates at a glance

```mermaid
graph TD
    schema["quent-schema\n─────────────\nData model: Schema, Entity,\nEvent, Record, Field, DataType\nVisitor walk + Cursor"]
    constraints["quent-constraints\n─────────────\nConstraint trait (named Visitor)\nvalidate() orchestrator\nReport"]
    fsm["quent-fsm\n─────────────\nFSM topology constraint\nEvent-order rules per entity"]
    reftarget["quent-ref-target\n─────────────\nEntityRef target constraint\nReference points at real entity"]
    reftree["quent-ref-tree\n─────────────\nTree-structure constraint\nReferences form one rooted tree"]

    constraints --> schema
    fsm --> schema
    fsm --> constraints
    reftarget --> schema
    reftarget --> constraints
    reftree --> schema
    reftree --> constraints
    reftree -. "requires (tree-ref\nmust be target-constrained)" .-> reftarget

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class schema,constraints core;
    class fsm,reftarget,reftree cons;
```

| Crate | Role | Depends on |
| --- | --- | --- |
| `quent-schema` | Dumb data model + Visitor walk | — |
| `quent-constraints` | Constraint trait + `validate()` orchestrator | `quent-schema` |
| `quent-fsm` | Enforces valid event ordering per entity | `quent-schema`, `quent-constraints` |
| `quent-ref-target` | Validates that an `EntityRef` names a real entity | `quent-schema`, `quent-constraints` |
| `quent-ref-tree` | Validates that tree-forming refs form one rooted tree | `quent-schema`, `quent-constraints`, *(reads)* `quent-ref-target` |

---

## 2. The two-layer design

```mermaid
graph LR
    subgraph layer1["Layer 1 · Foundation (stable, dumb)"]
        direction TB
        schema_data["Schema\nEntity · Event · Record\nField · DataType · Annotations"]
        visitor["Visitor trait\nCursor walk\nSchema::walk()"]
        schema_data ~~~ visitor
    end

    subgraph layer2["Layer 2 · Rules (pluggable, independent)"]
        direction TB
        c_trait["Constraint trait\n= Visitor + NAME"]
        validate["validate::<(A,B,C)>()\nOne walk · one Report"]
        fsm2["FsmConstraint"]
        rt2["RefTargetConstraint"]
        rtr2["RefTreeConstraint"]
        c_trait ~~~ validate ~~~ fsm2 ~~~ rt2 ~~~ rtr2
    end

    layer1 -->|"provides the walk"| layer2

    classDef found fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef rule fill:#2da44e,stroke:#136229,color:#fff;
    class layer1 found;
    class layer2 rule;
```

**Key insight.** The schema data model never learns domain rules. All domain
knowledge — "events must follow this order", "this reference must point at a
real entity", "these references must form a tree" — lives exclusively in
`Layer 2` crates. You add new rules by adding new crates; `quent-schema` never
changes.

---

## 3. `quent-schema` — the data model

### 3.1 Structure

```mermaid
classDiagram
    class Schema {
        name: Identifier
        entities: Map
        records: Map
        annotations: Annotations
    }
    class Entity {
        name: Identifier
        events: Map
        annotations: Annotations
    }
    class Event {
        name: Identifier
        cardinality: Once | Multi
        payload: Map~Field~
        annotations: Annotations
    }
    class Record {
        name: Identifier
        fields: Map
        annotations: Annotations
    }
    class Field {
        name: Identifier
        ty: DataType
        annotations: Annotations
    }
    class DataType {
        Bool · U8‥U64 · I8‥I64
        F32 · F64 · String · Uuid
        Option(DataType)
        List(DataType)
        Record(Identifier)
        DynamicRecord
        EntityRef
    }
    class Annotations {
        docs: Option~String~
        constraints: Map~name → Constraint~
        metadata: Map
    }

    Schema "1" o-- "*" Entity
    Schema "1" o-- "*" Record
    Entity "1" o-- "*" Event
    Event "1" o-- "*" Field
    Record "1" o-- "*" Field
    Field "1" --> "1" DataType
    Schema ..> Annotations
    Entity ..> Annotations
    Event ..> Annotations
    Field ..> Annotations
    Record ..> Annotations
    DataType ..> Annotations : EntityRef carries its own
```

### 3.2 The Visitor walk

`Schema::walk(visitor)` performs a **depth-first** traversal, calling
`visitor.visit(&cursor)` at every node *before* its children. The `Cursor`
carries the complete path from the schema root so a visitor always knows
*where* it is.

```mermaid
graph TD
    S["Schema"]
    S --> SA["Annotations"]
    S --> E["Entity"]
    E --> EA["Annotations"]
    E --> EV["Event"]
    EV --> EVA["Annotations"]
    EV --> F["Field → DataType"]
    F --> ER["EntityRef\n(inline annotations)"]
    F --> RC["Record(name)\n→ pointer, not inlined"]
    S --> R["Record (visited once, top-level)"]
    R --> RF["Field → DataType …"]

    classDef schema fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef hot fill:#bf3989,stroke:#6e1f50,color:#fff;
    class S,E,EV,R schema;
    class ER,RC hot;
```

> **Record walk nuance.** A `Record(name)` field is a *pointer*; the walker
> does **not** descend into the referenced record at the field site. Each
> top-level `Record` is visited exactly once from `Schema::records`. Constraints
> that need to "follow" records (like `quent-ref-tree`) must build their own
> linking graph during the walk.

---

## 4. `quent-constraints` — the trait and the orchestrator

### 4.1 What a Constraint is

```mermaid
classDiagram
    class Visitor {
        <<trait — quent-schema>>
        type Output
        visit(cursor: &Cursor)
        finish() → Output
    }
    class Constraint {
        <<trait — quent-constraints>>
        const NAME: &'static str
    }
    Constraint --|> Visitor : requires Visitor + Default

    class FsmConstraint {
        NAME = "quent.fsm.v1"
    }
    class RefTargetConstraint {
        NAME = "quent.ref-target.v1"
    }
    class RefTreeConstraint {
        NAME = "quent.ref-tree.v1"
    }

    FsmConstraint ..|> Constraint
    RefTargetConstraint ..|> Constraint
    RefTreeConstraint ..|> Constraint

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class Visitor,Constraint core;
    class FsmConstraint,RefTargetConstraint,RefTreeConstraint cons;
```

A `Constraint` is exactly **a `Visitor` with a unique string name**. The name
is the same key used in the schema's `Annotations::constraints` map
(`"quent.fsm.v1"`, `"quent.ref-tree.v1"`, …), so the orchestrator can match
annotations to constraint implementations.

### 4.2 Single-pass validation

`validate::<(A, B, C)>(&schema)` composes all constraints — plus two built-in
checks — into a tuple visitor and walks the schema **exactly once**.

```mermaid
sequenceDiagram
    participant U as Caller
    participant V as validate::<(Fsm, RefTree, RefTarget)>
    participant W as Schema::walk
    participant Bi as built-in checks
    participant Fsm as FsmConstraint
    participant RT as RefTreeConstraint
    participant TG as RefTargetConstraint

    U->>V: validate(&schema)
    V->>W: walk( (UnresolvedRefs + Unregistered + (Fsm, RefTree, RefTarget)) )

    loop every element, depth-first
        W->>Bi: visit(cursor)
        W->>Fsm: visit(cursor)
        W->>RT: visit(cursor)
        W->>TG: visit(cursor)
    end

    W->>Fsm: finish() → check state graph
    W->>RT: finish() → check_tree()
    W->>TG: finish()
    W-->>V: outputs tuple
    V-->>U: Report { unregistered, invalid_references, results }
```

**Always-on built-ins** (not optional):

| Built-in | What it checks |
| --- | --- |
| `UnresolvedReferences` | Every `Record(name)` field resolves to a real top-level record |
| `UnregisteredConstraints` | Annotation names in the schema that were *not* passed to `validate` |

The single-walk design is why each constraint is a `Visitor`: the orchestrator
just puts them in a tuple and hands the tuple to the walker.

---

## 5. `quent-ref-target` — the prerequisite constraint

`ref-target` answers one narrow question:

> **"This `EntityRef` claims to point at entity `T` — does `T` exist in the schema?"**

```mermaid
graph LR
    ER["EntityRef\nannotations"]
    ER -->|"quent.ref-target.v1\n= &quot;Cluster&quot;"| RT["RefTarget(Identifier)"]
    RT -->|"schema.entity(&quot;Cluster&quot;)?"| OK{exists?}
    OK -->|yes| PASS["✅ OK"]
    OK -->|no| ERR["❌ UnknownTarget"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    classDef err fill:#cf222e,stroke:#82071e,color:#fff;
    class ER core;
    class RT,OK,PASS cons;
    class ERR err;
```

`ref-target` is the **prerequisite** for `ref-tree`. You cannot build a typed
tree out of references that do not even name a valid entity type.

---

## 6. `quent-fsm` — the event-order constraint

`fsm` answers one narrow question:

> **"Do the events of this entity follow the declared FSM topology?"**

It reads an FSM topology from the entity's annotations, builds a state graph
(via petgraph), and validates:

- Every declared state is reachable from the initial state.
- There are no isolated cycles that can never be escaped.
- The topology itself is consistent (no dangling transitions, no duplicate
  states).

```mermaid
graph LR
    EA["Entity\nannotations"]
    EA -->|"quent.fsm.v1\n= topology JSON"| FSM["FsmConstraint\nstate graph (petgraph)"]
    FSM -->|"Bfs + tarjan_scc"| CHK{valid?}
    CHK -->|yes| PASS2["✅ OK"]
    CHK -->|no| ERR2["❌ FsmError"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    classDef err fill:#cf222e,stroke:#82071e,color:#fff;
    class EA core;
    class FSM,CHK,PASS2 cons;
    class ERR2 err;
```

---

## 7. `quent-ref-tree` — the tree-structure constraint

`ref-tree` answers one narrow question:

> **"Do the entity references annotated as tree-forming together describe a
> single tree rooted at exactly one entity?"**

Its five requirements:

```mermaid
graph TD
    R1["Req 1 · Exactly ONE root\n(an entity whose events carry no tree-ref)"]
    R2["Req 2 · Each non-root entity has\n≤1 tree-ref per event / record"]
    R3["Req 3 · Every tree-ref must also\nbe target-constrained (ref-target)"]
    R4["Req 4 · All of one entity's tree-refs\npoint at the SAME parent type"]
    R5["Req 5 · Every entity can follow\ntree-refs up to the root"]

    R3 --> R1
    R2 --> R4
    R1 --> R4
    R4 --> R5

    classDef req fill:#2da44e,stroke:#136229,color:#fff;
    class R1,R2,R3,R4,R5 req;
```

Because the `Record` walk nuance (§3.2) means records aren't followed inline,
`ref-tree` builds its own internal directed graph during the walk and collapses
it in `finish()`:

```mermaid
flowchart TD
    start([finish called]) --> used{any tree-ref\nactually used?}
    used -->|no| ok0([✅ Ok — no tree to form])
    used -->|yes| bad{any NotTargetConstrained\nor UnknownTarget?}
    bad -->|yes| rep([report those errors only])
    bad -->|no| col[For each entity:\nwalk events + records → unique parent entities]

    col --> cnt{unique parents\nper entity?}
    cnt -->|0| root[mark as root]
    cnt -->|1| child[mark non-root,\nadd parent→child edge]
    cnt -->|2+| conf[❌ ConflictingParents · Req 4]

    root --> roots{how many roots?}
    roots -->|0| nr[❌ NoRoot · Req 1]
    roots -->|2+| mr[❌ MultipleRoots · Req 1]
    roots -->|1| bfs[BFS from root over\ncollapsed entity tree]
    child --> bfs

    bfs --> reach{every non-root\nreached?}
    reach -->|no| unr[❌ Unreachable · Req 5]
    reach -->|yes| done([✅ Ok — valid tree])

    classDef ok fill:#2da44e,stroke:#136229,color:#fff;
    classDef err fill:#cf222e,stroke:#82071e,color:#fff;
    class ok0,done,bfs ok;
    class rep,conf,nr,mr,unr err;
```

---

## 8. How the crates interplay at runtime

### 8.1 Constraint composition

All three constraints are `Visitor + Constraint`. They compose into a tuple
and are passed to `Schema::walk` in a **single pass**:

```mermaid
graph LR
    subgraph schema_box["quent-schema (one walk)"]
        W["Schema::walk"]
    end

    subgraph constraints_box["quent-constraints (orchestrates)"]
        VAL["validate::<(Fsm, RefTree, RefTarget)>"]
        BI["built-in checks\n(UnresolvedRefs, Unregistered)"]
    end

    subgraph impls_box["constraint impls (execute in lock-step)"]
        FSM["FsmConstraint\n• reads Entity annotations\n• builds per-entity state graph"]
        RTR["RefTreeConstraint\n• reads EntityRef annotations\n• builds entity/event/record graph"]
        TGT["RefTargetConstraint\n• reads EntityRef annotations\n• verifies entity name exists"]
    end

    VAL -->|one walk| W
    W --> BI
    W --> FSM
    W --> RTR
    W --> TGT

    FSM -->|"finish()"| FSMOut["Result&lt;(), FsmError&gt;"]
    RTR -->|"finish()"| RTROut["Result&lt;(), RefTreeError&gt;"]
    TGT -->|"finish()"| TGTOut["Result&lt;(), RefTargetError&gt;"]

    FSMOut & RTROut & TGTOut --> REP["Report { results: (Fsm, RefTree, RefTarget) }"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class schema_box,W,VAL,BI core;
    class FSM,RTR,TGT,FSMOut,RTROut,TGTOut,REP cons;
```

### 8.2 `ref-tree` depends on `ref-target` (not on the constraint, but on its data)

`ref-tree` does not *call* `RefTargetConstraint` during its walk. Instead, it
**reads the same annotation data** that `ref-target` writes, using the shared
`RefTarget::from_annotations(...)` helper. This is why requirement 3 exists:
if a tree-forming `EntityRef` lacks the `ref-target` annotation, `ref-tree`
has no parent to point at.

```mermaid
graph LR
    ANN["EntityRef Annotations\nquent.ref-tree.v1 = marker\nquent.ref-target.v1 = &quot;Node&quot;"]
    ANN -->|"read by"| RTR2["RefTreeConstraint\nuses target identifier\nto build internal graph"]
    ANN -->|"read by"| TGT2["RefTargetConstraint\nverifies &quot;Node&quot; exists\nin schema.entities"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class ANN core;
    class RTR2,TGT2 cons;
```

### 8.3 `fsm` and `ref-tree` are independent peers

They never call each other. Run together, they validate **two orthogonal
dimensions** of the same schema in one pass:

```mermaid
graph LR
    subgraph time_axis["FSM · time axis (how one entity evolves)"]
        FS["FsmConstraint\n'what events can\nfollow what states'"]
    end
    subgraph space_axis["Ref-tree · space axis (how entities relate)"]
        RT3["RefTreeConstraint\n'who owns whom\nin the hierarchy'"]
    end
    SCH["Schema"] --> FS & RT3

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class SCH core;
    class FS,RT3 cons;
```

| | `quent-fsm` | `quent-ref-tree` |
| --- | --- | --- |
| Validates | **temporal order** of one entity's events | **spatial hierarchy** across all entities |
| Annotation lives on | `Entity` (FSM topology JSON) | `EntityRef` (marker + target) |
| Internal model | per-entity state graph (petgraph) | whole-schema entity/event/record graph |
| Graph algorithm | BFS + Tarjan SCC | DFS collapse + BFS reachability |
| Output | `Result<(), FsmError>` | `Result<(), RefTreeError>` |

---

## 9. Real-world example: a Kubernetes-style container platform

### 9.1 The domain hierarchy

A container platform has a natural ownership tree. Each entity is *owned by*
or *part of* its parent:

```mermaid
graph TD
    Cluster["Cluster\n(root — owns nothing)"]
    Node["Node\nis part of a Cluster"]
    Pod["Pod\nis scheduled on a Node"]
    Container["Container\nis part of a Pod"]

    Cluster --> Node --> Pod --> Container

    classDef root fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef child fill:#2da44e,stroke:#136229,color:#fff;
    class Cluster root;
    class Node,Pod,Container child;
```

### 9.2 The event lifecycle of each entity (FSM dimension)

Each entity has a defined set of lifecycle events. `quent-fsm` enforces the
valid ordering:

```mermaid
stateDiagram-v2
    direction LR
    state "Cluster" as CL {
        [*] --> Provisioned : provisioned
        Provisioned --> Decommissioned : decommissioned
    }
    state "Node" as ND {
        [*] --> Joined : joined
        Joined --> Drained : drained
        Joined --> Failed : failed
        Drained --> [*]
        Failed --> [*]
    }
    state "Pod" as PD {
        [*] --> Scheduled : scheduled
        Scheduled --> Running : running
        Running --> Succeeded : succeeded
        Running --> Failed2 : failed
        Succeeded --> [*]
        Failed2 --> [*]
    }
    state "Container" as CT {
        [*] --> Started : started
        Started --> Exited : exited
        Exited --> [*]
    }
```

### 9.3 The tree-forming references (ref-tree + ref-target dimension)

Each non-root entity's first event carries a **tree-forming reference** to its
parent. That field's `EntityRef` is annotated with both `quent.ref-tree.v1`
(tree-forming marker) and `quent.ref-target.v1` (the parent entity name).

```mermaid
graph LR
    subgraph Node_event["Event: Node.joined"]
        NF["field 'cluster'\ntype: EntityRef\nref-tree.v1 = marker\nref-target.v1 = &quot;Cluster&quot;"]
    end
    subgraph Pod_event["Event: Pod.scheduled"]
        PF["field 'node'\ntype: EntityRef\nref-tree.v1 = marker\nref-target.v1 = &quot;Node&quot;"]
    end
    subgraph Container_event["Event: Container.started"]
        CF["field 'pod'\ntype: EntityRef\nref-tree.v1 = marker\nref-target.v1 = &quot;Pod&quot;"]
    end

    NF -->|"tree-ref points at"| CL2["Entity: Cluster"]
    PF -->|"tree-ref points at"| ND2["Entity: Node"]
    CF -->|"tree-ref points at"| PD2["Entity: Pod"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class CL2,ND2,PD2 core;
    class NF,PF,CF cons;
```

### 9.4 Building the schema with the builder API

```rust
use quent_constraints::{validate};
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::{RefTreeConstraint, RefTreeError};
use quent_fsm::FsmConstraint;
use quent_schema::{
    Cardinality, DataType, Identifier,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, SchemaBuilder},
};

fn id(s: &str) -> Identifier {
    Identifier::try_new(s).unwrap()
}

/// Builds a DataType::EntityRef that is BOTH tree-forming and target-constrained.
/// Both constraints read from the same annotations map.
fn parent_ref(parent: &str) -> DataType {
    let target = serde_json::to_string(&id(parent)).unwrap();
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .constraint(RefTreeConstraint::NAME, None)   // tree-forming marker
            .unwrap()
            .constraint(RefTargetConstraint::NAME, Some(target)) // target name
            .unwrap()
            .build(),
    }
}

fn build_platform_schema() -> quent_schema::Schema {
    // Cluster: the root. No tree-forming refs in any of its events.
    let cluster = EntityBuilder::new(id("Cluster"))
        .event(EventBuilder::new(id("provisioned"), Cardinality::Once).build())
        .unwrap()
        .event(EventBuilder::new(id("decommissioned"), Cardinality::Once).build())
        .unwrap()
        .build();

    // Node: first event carries a tree-ref to Cluster.
    let node = EntityBuilder::new(id("Node"))
        .event(
            EventBuilder::new(id("joined"), Cardinality::Once)
                .field(quent_schema::Field::new(
                    id("cluster"),
                    parent_ref("Cluster"),
                    Default::default(),
                ))
                .unwrap()
                .build(),
        )
        .unwrap()
        .event(EventBuilder::new(id("drained"), Cardinality::Once).build())
        .unwrap()
        .event(EventBuilder::new(id("failed"), Cardinality::Once).build())
        .unwrap()
        .build();

    // Pod: tree-ref to Node on its "scheduled" event.
    let pod = EntityBuilder::new(id("Pod"))
        .event(
            EventBuilder::new(id("scheduled"), Cardinality::Once)
                .field(quent_schema::Field::new(
                    id("node"),
                    parent_ref("Node"),
                    Default::default(),
                ))
                .unwrap()
                .build(),
        )
        .unwrap()
        .event(EventBuilder::new(id("running"), Cardinality::Once).build())
        .unwrap()
        .event(EventBuilder::new(id("succeeded"), Cardinality::Once).build())
        .unwrap()
        .event(EventBuilder::new(id("failed"), Cardinality::Once).build())
        .unwrap()
        .build();

    // Container: tree-ref to Pod on its "started" event.
    let container = EntityBuilder::new(id("Container"))
        .event(
            EventBuilder::new(id("started"), Cardinality::Once)
                .field(quent_schema::Field::new(
                    id("pod"),
                    parent_ref("Pod"),
                    Default::default(),
                ))
                .unwrap()
                .build(),
        )
        .unwrap()
        .event(EventBuilder::new(id("exited"), Cardinality::Once).build())
        .unwrap()
        .build();

    SchemaBuilder::new(id("K8sPlatform"))
        .entities([cluster, node, pod, container])
        .unwrap()
        .build()
}
```

### 9.5 Running all three constraints together

```rust
fn main() {
    let schema = build_platform_schema();

    // A single walk validates FSM order, ref-target existence, and ref-tree shape.
    let report = validate::<(FsmConstraint, RefTreeConstraint, RefTargetConstraint)>(&schema);

    let (fsm_result, reftree_result, reftarget_result) = report.results;

    match fsm_result {
        Ok(()) => println!("✅ all entity FSM topologies are valid"),
        Err(e) => eprintln!("❌ FSM violation: {e}"),
    }
    match reftree_result {
        Ok(()) => println!("✅ entity references form a valid tree rooted at Cluster"),
        Err(RefTreeError::Multiple(errs)) => {
            eprintln!("❌ {} tree violations:", errs.len());
            for e in errs { eprintln!("  - {e}"); }
        }
        Err(e) => eprintln!("❌ tree violation: {e}"),
    }
    match reftarget_result {
        Ok(()) => println!("✅ all EntityRef targets are valid"),
        Err(e) => eprintln!("❌ ref-target violation: {e}"),
    }
}
```

### 9.6 What the constraint sees during the walk

During the walk `ref-tree` builds this internal graph…

```mermaid
graph LR
    subgraph Entities["Entity nodes"]
        ECl["Entity(Cluster)"]
        ENo["Entity(Node)"]
        EPo["Entity(Pod)"]
        ECo["Entity(Container)"]
    end

    subgraph Events["Event nodes"]
        EvProv["Event(Cluster, provisioned)"]
        EvJoin["Event(Node, joined)"]
        EvSched["Event(Pod, scheduled)"]
        EvStart["Event(Container, started)"]
    end

    ECl --> EvProv
    ENo --> EvJoin
    EPo --> EvSched
    ECo --> EvStart

    EvJoin  -->|"tree-ref"| ECl
    EvSched -->|"tree-ref"| ENo
    EvStart -->|"tree-ref"| EPo

    classDef entity fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef event fill:#d4a015,stroke:#7d5b00,color:#fff;
    class ECl,ENo,EPo,ECo entity;
    class EvProv,EvJoin,EvSched,EvStart event;
```

…and `finish()` collapses it to the clean entity tree:

```mermaid
graph TD
    Cluster2["Cluster (root)"]
    Node2["Node"]
    Pod2["Pod"]
    Container2["Container"]

    Cluster2 --> Node2 --> Pod2 --> Container2

    classDef root fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef child fill:#2da44e,stroke:#136229,color:#fff;
    class Cluster2 root;
    class Node2,Pod2,Container2 child;
```

### 9.7 How violations map to errors

```mermaid
graph TD
    subgraph CP["❌ ConflictingParents (Req 4)"]
        CP1["Pod.scheduled → Node"]
        CP2["Pod.evicted   → Cluster"]
        CP1 -. "same entity, different\nparent types" .-> CPX["ConflictingParents"]
        CP2 -. .-> CPX
    end

    subgraph MR["❌ MultipleRoots (Req 1)"]
        MR1["Cluster (no parent)"]
        MR2["Region  (no parent)"]
        MR1 -. "both have zero parents" .-> MRX["MultipleRoots"]
        MR2 -. .-> MRX
    end

    subgraph UR["❌ Unreachable (Req 5)"]
        UR1["A → B"]
        UR2["B → A"]
        UR1 -. "cycle, never reaches root" .-> URX["Unreachable"]
        UR2 -. .-> URX
    end

    subgraph NT["❌ NotTargetConstrained (Req 3)"]
        NT1["EntityRef has\nref-tree.v1 marker"]
        NT2["but NO\nref-target.v1"]
        NT1 -. "missing prerequisite" .-> NTX["NotTargetConstrained"]
        NT2 -. .-> NTX
    end

    subgraph NR["❌ NoRoot (Req 1)"]
        NR1["Every entity has\nat least one tree-ref"]
        NR1 -. "nowhere to start BFS" .-> NRX["NoRoot"]
    end

    classDef err fill:#cf222e,stroke:#82071e,color:#fff;
    class CPX,MRX,URX,NTX,NRX err;
```

| Trigger | Error |
| --- | --- |
| An entity's events point at two different parent types | `ConflictingParents` |
| Two entities have zero tree-refs in all their events | `MultipleRoots` |
| Every entity has at least one tree-ref (no root exists) | `NoRoot` |
| An entity's events form a cycle that never touches the root | `Unreachable` |
| A `ref-tree` marker exists but `ref-target` annotation is missing | `NotTargetConstrained` |
| A `ref-target` annotation names an entity that isn't in the schema | `UnknownTarget` |
| An event carries more than one tree-forming ref | `MultiplePerEvent` |

---

## 10. Why this architecture is nice

```mermaid
graph LR
    subgraph stable["stable (never changes)"]
        S["quent-schema\ndata + walk"]
        C["quent-constraints\ntrait + orchestrator"]
    end

    subgraph pluggable["pluggable (add new crates freely)"]
        F["quent-fsm"]
        RT["quent-ref-target"]
        RTR["quent-ref-tree"]
        X["quent-… (future)"]
    end

    C --> S
    F & RT & RTR & X --> C

    S -. "opaque annotation blobs" .-> F & RT & RTR & X
    F & RT & RTR & X -->|"share one walk"| REP2["Report"]

    classDef core fill:#1f6feb,stroke:#0b3a8c,color:#fff;
    classDef cons fill:#2da44e,stroke:#136229,color:#fff;
    class stable,S,C core;
    class pluggable,F,RT,RTR,X,REP2 cons;
```

**Separation of concerns.** The schema never learns domain rules. New rules ship
as new crates; `quent-schema` never changes.

**Forward compatibility.** An unknown annotation name is *reported*
(`UnregisteredConstraints`), never silently ignored. Codegen can emit a working
API even before understanding a new constraint.

**Single-pass efficiency.** `validate::<(A, B, C)>` composes visitors into a
tuple and walks the schema once, regardless of how many constraints are active.

**Composability.** `ref-tree` reusing `ref-target`'s annotation data shows
constraints can layer on each other instead of duplicating logic.

**Orthogonality.** `fsm` and `ref-tree` describe the same schema from two
independent dimensions — time and space — without any coupling between them.

---

## 11. Adding your own constraint

Any crate can participate by implementing `Visitor + Constraint`:

```rust
use quent_schema::Visitor;
use quent_constraints::Constraint;

/// Example: enforce that every Entity has at least one event.
#[derive(Default)]
pub struct NonEmptyEntitiesConstraint {
    violations: Vec<String>,
}

impl Visitor for NonEmptyEntitiesConstraint {
    type Output = Result<(), Vec<String>>;

    fn visit(&mut self, cursor: &quent_schema::Cursor) {
        if let Some(entity) = cursor.as_entity() {
            if entity.events.is_empty() {
                self.violations.push(entity.name.to_string());
            }
        }
    }

    fn finish(self) -> Self::Output {
        if self.violations.is_empty() { Ok(()) } else { Err(self.violations) }
    }
}

impl Constraint for NonEmptyEntitiesConstraint {
    const NAME: &'static str = "my-org.non-empty-entities.v1";
}

// Usage: compose it in alongside the built-in constraints.
// let report = validate::<(FsmConstraint, RefTreeConstraint, NonEmptyEntitiesConstraint)>(&schema);
```

The new constraint is automatically included in the single-walk pass with no
changes required to `quent-schema` or `quent-constraints`.
