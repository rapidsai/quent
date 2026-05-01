use std::marker::PhantomData;

use quent_model_macros::Entity;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

// User-facing types used for modeling

// An event that can only be emitted <= 1 time per entity
pub struct Once<T> {
    _phantom: PhantomData<T>,
}

// An event that can be emitted >= 0 times per entity
pub struct Multi<T> {
    _phantom: PhantomData<T>,
}

// Every entity has a unique id.
// For instrumentation:
pub trait EntityHandle {
    fn id(&self) -> Uuid;
}

// Every entity has a unique id.
// For analysis:
pub trait EntityModel {
    fn id(&self) -> Uuid;
    fn type_name() -> String;
}

// An event has a timestamp and a payload
pub struct Event<T> {
    pub id: Uuid,
    pub timestamp: TimeUnixNanoSec,
    pub payload: T,
}

#[derive(Debug)]
pub struct ObserverError;
impl std::fmt::Display for ObserverError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl std::error::Error for ObserverError {}

mod one_shot_empty {
    use super::*;

    mod model {
        use super::*;

        // Most trivial entity. Just emits one event without attributes. Thus
        // its only properties are a UUID and a timestamp for its single event.
        #[derive(Entity)]
        pub struct OneShotEmpty;
    }

    mod events {
        // nothing here, Event<()> already captures everything.
    }

    mod instrumentation {
        use super::*;

        // Each entity gets a dedicated observer in the client application.
        //
        // An observer sinks events and exports them. The observer can e.g.
        // batch events, or even validate entire entities before sending them
        // out, producing error logs etc. when e.g. state transition violation
        // occur. Another example is writing events to a Parquet file per entity
        // without requiring one single schema for the entire model. This makes
        // the schemas simpler, easier to write files, and easier inspect them
        // manually, or with ad-hoc scripts.
        //
        // From this observer, each entity you create can clone the sender into
        // a handle. In this specific case this is not necessary, since we emit
        // exactly one event per entity, so we don't need a handle that keeps
        // entity state (as far as emitting events goes). Examples of a handle
        // are shown in other entities below.
        pub struct OneShotEmptyObserver {}

        impl OneShotEmptyObserver {
            // Returns a result with the uuid of the entity. Since this is a
            // single one shot event entity, it just returns a Uuid.
            pub fn one_shot_empty(&self) -> Result<Uuid, ObserverError> {
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;
        // Generate traits of entity models in analysis. These traits exist for
        // convenience and could be implemented by various entity containers,
        // e.g. plain Rust types, column-oriented formats such as Arrow through
        // a reference to some row representing the entity, or a time-series
        // database, etc.
        //
        // Single-event entity, so if the event ever arrived, we know its
        // timestamp.
        pub trait OneShotEmptyModel: EntityModel {
            fn one_shot_empty(&self) -> Event<()>;
        }
    }
}

mod one_shot_with_attribs {
    use super::*;

    mod model {
        use super::*;

        // Single-event entity. Just emits one event with attributes of this
        // struct.
        #[derive(Entity)]
        pub struct OneShotWithAttribs {
            // is it fine that this implicitly becomes the entire event vs.
            // multi event syntax below? alternative is to require Entity to
            // always have at least one Once<Attributes> or Multi<Attributes>
            // field, but for single-event entities, either the entity name or
            // field name needs to be chosen for the observer api call to
            // produce this event.
            foo: u64,
            bar: String,
        }
    }

    mod events {
        // the OneShotWithAttribs struct is the event
    }

    mod instrumentation {
        use super::*;
        pub struct OneShotWithAttribsObserver {
            // holds sender
        }
        impl OneShotWithAttribsObserver {
            // Same as OneShotEmpty, does not take &self since there is no
            // state, so we don't need an entity handle yet.
            pub fn one_shot_with_attribs(
                _attributes: super::model::OneShotWithAttribs,
            ) -> Result<Uuid, ObserverError> {
                // emits event
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;
        // Still single event entity. If it ever arrived, we know it in its
        // entirety.
        pub trait OneShotWithAttribsModel: EntityModel {
            fn one_shot_with_attribs() -> Event<super::model::OneShotWithAttribs>; // hence this is not optional
        }
    }
}

mod multi_one_shot {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Entity)]
        pub struct MultiOneShot {
            a: Once<X>, // field name becomes the event name
            b: Once<Y>,
            c: Once<Y>, // same attributes type as b, but semantically different event
            d: Once<()>,
        }
    }

    mod events {
        pub enum MultiOneShotEvent {
            A(super::model::X),
            B(super::model::Y),
            C(super::model::Y), // same attributes type as B, but semantically different event
            D,
        }
    }

    mod instrumentation {
        use super::*;

        pub struct MultiOneShotObserver {}

        impl MultiOneShotObserver {
            fn handle(&self) -> Result<MultiOneShotHandle, ObserverError> {
                // Returns a new handle, generating a new UUID and cloning the
                // sender, all event flags unset.
                // Could error out if the channel is closed etc.
                todo!()
            }
        }

        pub struct MultiOneShotHandle {
            // holds entity uuid
            // holds sender
            // holds flags on what events have already been emitted to
            // detect logic errors
        }

        impl EntityHandle for MultiOneShotHandle {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl MultiOneShotHandle {
            fn a(&self, _attributes: super::model::X) -> Result<(), ObserverError> {
                // errors out if the event was previously submitted already
                //
                // emits event, flags this as emitted
                todo!()
            }
            fn b(&self, _attributes: super::model::Y) -> Result<(), ObserverError> {
                todo!()
            }
            // same attributes type as b(), but semantically different event
            fn c(&self, _attributes: super::model::Y) -> Result<(), ObserverError> {
                todo!()
            }
            fn d(&self) {
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;

        // If at least one event arrived, we know this entity exists. But it
        // could have been in any order, and events may not have been sent, so
        // these are all optional.
        pub trait MultiOneShotModel: EntityModel {
            fn a() -> Option<Event<super::model::X>>;
            fn b() -> Option<Event<super::model::Y>>;
            fn c() -> Option<Event<super::model::Y>>;
            fn d() -> Option<Event<()>>;
        }
    }
}

mod one_multi_shot {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        #[derive(Entity)]
        pub struct OneMultiShot {
            a: Multi<X>,
        }
    }

    mod events {
        pub enum OneMultiShotEvent {
            A(super::model::X),
        }
    }

    mod instrumentation {
        use super::*;

        pub struct OneMultiShotObserver {}

        impl OneMultiShotObserver {
            fn handle(&self) -> Result<OneMultiShotHandle, ObserverError> {
                todo!()
            }
        }

        pub struct OneMultiShotHandle {
            // holds entity UUID
            // holds sender
            // doesn't hold any flags
        }

        impl EntityHandle for OneMultiShotHandle {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl OneMultiShotHandle {
            fn a(&self, _attributes: super::model::X) {
                // emits event
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;

        pub trait OneMultiShotModel: EntityModel {
            fn a() -> impl Iterator<Item = Event<super::model::X>>;
        }
    }
}

mod multi_multi_shot {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Entity)]
        pub struct MultiMulti {
            a: Multi<X>,
            b: Multi<X>,
            c: Multi<Y>,
        }
    }

    mod events {
        pub enum XEvent {
            A(super::model::X),
            B(super::model::X),
            C(super::model::Y),
        }
    }

    mod instrumentation {
        use super::*;

        pub struct MultiMultiObserver {}

        impl MultiMultiObserver {
            fn handle(&self) -> Result<MultiMultiHandle, ObserverError> {
                todo!()
            }
        }

        pub struct MultiMultiHandle {
            // holds uuid
            // holds sender
        }

        impl EntityHandle for MultiMultiHandle {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl MultiMultiHandle {
            fn a(&self, _attributes: super::model::X) {
                // emits event
                todo!()
            }
            fn b(&self, _attributes: super::model::X) {
                // emits event
                todo!()
            }
            fn c(&self, _attributes: super::model::Y) {
                // emits event
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;

        pub trait MultiMultiShotModel: EntityModel {
            fn a() -> impl Iterator<Item = Event<super::model::X>>;
            fn b() -> impl Iterator<Item = Event<super::model::X>>;
            fn c() -> impl Iterator<Item = Event<super::model::Y>>;
        }
    }
}

mod mixed {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Entity)]
        pub struct Mixed {
            a: Once<X>,
            b: Multi<Y>,
        }
    }

    mod events {
        pub enum MixedEvent {
            A(super::model::X),
            B(super::model::Y),
        }
    }

    mod instrumentation {
        use super::*;

        pub struct MixedObserver {}

        impl MixedObserver {
            fn handle(&self) -> Result<MixedHandle, ObserverError> {
                todo!()
            }
        }

        pub struct MixedHandle {
            // holds entity uuid
            // holds sender
            // holds flags for one-shot events, so in this case only for event A
        }

        impl EntityHandle for MixedHandle {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl MixedHandle {
            pub fn a(&self, _attributes: super::model::X) -> Result<(), ObserverError> {
                // emits event
                // errors out if event was already sent
                todo!()
            }

            pub fn b(&self, _attributes: super::model::Y) {
                // emits event
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;

        pub trait MixedModel: EntityModel {
            fn a() -> Option<Event<super::model::X>>;
            fn b() -> impl Iterator<Item = Event<super::model::X>>;
        }
    }
}

// Some invalid things that should produce compilation errors
mod invalid {
    use super::*;

    mod model {
        use super::*;

        // Can't have plain fields if there are any Once or Many typed fields.
        // Plain fields are only allowed for single, one-shot event entities.

        #[derive(Entity)]
        pub struct Invalid {
            plain: u64,
            a: Once<()>,
        }
    }
}
