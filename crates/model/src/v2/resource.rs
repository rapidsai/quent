use std::marker::PhantomData;

use uuid::Uuid;

// user facing model declaration types:

struct Capacity<T> {
    _phantom: PhantomData<T>,
}

// user examples:

// Unit resource
#[derive(Resource)] // derive macro
struct Thread;

// Resource with occupancy capacity
#[derive(Resource)]
struct Memory {
    bytes: Capacity<u64>,
    // what to do with arbitrary other attributes?
    // will they go on initializing state, if is that obvious to the users?
    // maybe: foo: Atrribute<T> ?
}

// Resource with resizable capacity
#[derive(Resource)]
struct MemoryResizable {
    #[quent(resizable)] // means it will have the operating <-> resizing state+transitions
    bytes: Capacity<u64>,
}

// Resource with potentially unbounded capacity
#[derive(Resource)]
struct MemoryUnbounded {
    #[quent(unbounded)]
    bytes: Capacity<u64>,
}

// Resource with rate capacity
//
// For the resource operating state transition, the user is going to convey the
// maximum number of items per unit of time that the resource supports if the
// capacity is a "rate" kind of capacity. If it is "unbounded", then the user is
// not required to set the capacity bounds, but they can set the capacity
// bounds, if they know. Thus we need the transition to the resource operating
// state to make that clear by its arguments, e.g. operating(bytes_per_second:
// f64).
//
// For an FSM entering a state in which this resource is used, the amount of a
// "rate" kind capacity it uses is derived from the user supplying the amount of
// items and the amount of time that amount of items were used. But, the time
// dimension is captured implicitly by the state transition events. So for an
// FSM transition with a usage of this resource, say it is a network packet
// transferred over some network channel, in the FSM transition event, they only
// have to supply the size of the packet, rather than wait for the transition to
// complete and calculate give the "bytes per second" number explicitly
// afterwards. Thus the fsm transition event API will simply be something like
// transfer(bytes: u64).
#[derive(Resource)]
struct Channel {
    #[quent(rate, unbounded)]
    bytes: Capacity<u64>,
}

// user facing result (rust):

struct MemoryObserver {
    // holds sender to mpsc event channel
}
impl MemoryObserver {
    fn init() -> MemoryHandle {
        // clones sender into handle
        todo!()
    } // emits event and creates a handle
}

struct MemoryHandle {
    id: Uuid,
    // holds sender to event channel of the observer
}

impl MemoryHandle {
    fn operating(self, bytes: u64) -> Self {
        todo!()
        // emits event
    }
    fn finalizing(self) -> Self {
        todo!()
        // emits event
    }
    fn exit(self) {
        todo!()
        // emits event
    }
}

struct MemoryResizableObserver {}
impl MemoryResizableObserver {
    fn init() -> MemoryResizableHandle {
        todo!()
    }
}
struct MemoryResizableHandle {}
impl MemoryResizableHandle {
    fn operating(self, bytes: u64) -> Self {
        todo!()
    }
    fn resizing(self) -> Self {
        todo!()
    }
    fn finalizing(self) -> Self {
        todo!()
    }
    fn exit(self) -> Self {
        todo!()
    }
}

struct MemoryUnboundedObserver {}
impl MemoryUnboundedObserver {
    fn init() -> MemoryUnboundedHandle {
        todo!()
    }
}
struct MemoryUnboundedHandle {}
impl MemoryUnboundedHandle {
    fn operating(self, bytes: Option<u64>) -> Self {
        todo!()
    }
    fn finalizing(self) -> Self {
        todo!()
    }
    fn exit(self) {
        todo!()
    }
}

struct ChannelObserver {}
impl ChannelObserver {
    fn init() -> ChannelHandle {
        todo!()
    }
}
struct ChannelHandle {}
impl ChannelHandle {
    // Maybe needs f64 ?
    fn operating(self, bytes_per_second: Option<u64>) -> Self {
        todo!()
    }
    fn finalizing(self) -> Self {
        todo!()
    }
    fn exit() {
        todo!()
    }
}

// in macros/codegen, not required, just a brainstorm, may need to look completely different:

enum ValueType {
    U64,
}

enum CapacityKind {
    Occupancy,
    Rate,
}

struct CapacityDecl {
    name: String,
    kind: CapacityKind,
    typ: ValueType,
    resizeable: bool,
    unbounded: bool,
}

trait ResourceDecl {
    fn capacities() -> impl Iterator<Item = CapacityDecl>;
}

impl ResourceDecl for Memory {
    fn capacities() -> impl Iterator<Item = CapacityDecl> {
        [CapacityDecl {
            name: "bytes".to_owned(),      // from the field name
            kind: CapacityKind::Occupancy, // implicit, occupancy unless there is #[quent(rate)]
            typ: ValueType::U64,           // from the field type
            resizeable: false,             // from #[quent(resizeable)]
            unbounded: false,              // from field type (when using Option<T>)
        }]
        .into_iter()
    }
}

impl ResourceDecl for MemoryResizable {
    fn capacities() -> impl Iterator<Item = CapacityDecl> {
        [CapacityDecl {
            name: "bytes".to_owned(),
            kind: CapacityKind::Occupancy,
            typ: ValueType::U64,
            resizeable: true,
            unbounded: false,
        }]
        .into_iter()
    }
}

impl ResourceDecl for MemoryUnbounded {
    fn capacities() -> impl Iterator<Item = CapacityDecl> {
        [CapacityDecl {
            name: "bytes".to_owned(),
            kind: CapacityKind::Occupancy,
            typ: ValueType::U64,
            resizeable: false,
            unbounded: true,
        }]
        .into_iter()
    }
}
