// user facing model declaration types:

use uuid::Uuid;

// special marker type for exit states
struct Exit;

#[derive(Resource)]
struct Cache {
    entries: Capacity<u64>,
    bytes: Capacity<u64>,
}

#[derive(Resource)]
struct Thread;

#[derive(State)]
struct Cached {
    cache: Usage<Cache>,
}

#[derive(State)]
struct Evicting {
    cache: Usage<Cache>,
    thread: Usage<Thread>,
}

#[derive(Fsm)]
struct Item {
    #[quent(entry, to=evicting)]
    cached: State<Cached>,
    #[quent(exit)]
    evicting: State<Evicting>,
}

// user facing result (rust):

// Not really needed as this is a unit resource:
// struct ThreadUsage<'a> {
//     handle: &'a ThreadHandle, // or plain UUID for CXX?
// }

struct CacheUsage<'a> {
    handle: &'a CacheHandle, // or plain UUID for CXX?
    entries: Option<u64>,
    bytes: Option<u64>,
}
struct ItemObserver {}
impl ItemObserver {
    fn cached(cache: CacheUsage) -> ItemHandle {
        todo!()
    }
}
// Could technically also create a type-safe handles that CAN'T transition into
// the wrong state at compile time. For resource and other small FSMs, this is
// fine, but for arbitrary FSMs with many possible transitions this may explode
// on the CXX side, since there we have to monomorphize.
struct ItemHandle {}
impl ItemHandle {
    fn serializing(self, thread: &ThreadHandle) -> Self {
        todo!()
    }
    fn exit(self) {
        todo!()
    }
}
