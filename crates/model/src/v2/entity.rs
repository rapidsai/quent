use std::marker::PhantomData;

use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

// user facing types used for modeling

// An event that can only be emitted <= 1 time per entity
struct Once<T> {
    _phantom: PhantomData<T>,
}

// An event that can be emitted >= 0 times per entity
struct Multi<T> {
    _phantom: PhantomData<T>,
}

mod one_shot_empty {
    use super::*;

    mod model {
        use super::*;
        // Most trivial entity. Just emits one event without attributes.
        // Thus its only properties are a UUID and a timestamp.
        #[derive(Entity)]
        struct OneShotEmpty;
    }

    mod events {
        use super::*;
        // Do we really need this? Could just be ()
        struct OneShotEmptyEvent;
    }

    mod instrumentation {
        use super::*;
        struct OneShotEmptyObserver {
            // holds sender
        }
        impl OneShotEmptyObserver {
            fn OneShotEmpty(&self) {
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;
        // Single-event entity, so if the event ever arrived, we know its
        // timestamp. No attributes.
        trait OneShotEmptyModel: quent_analyzer::Entity {
            fn one_shot_empty() -> TimeUnixNanoSec;
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
        struct OneShotWithAttribs {
            // is it fine that this implicitly becomes the entire event vs.
            // multi event syntax below? alternative is to require Entity to
            // always have at least one Once<Attributes> or Multi<Attributes>
            // field, but for single-event entities, either the entity name or
            // field name needs to be chosen for the observer api call to
            // produce this event.
            foo: u64,
            bar: String
        }
    }

    mod events {
        use super::*;
        // Do we really need this? Could just be OneShotWithAttribs
        struct OneShotWithAttribs?Event? {
            foo: u64,
            bar: String
        }
    }

    mod instrumentation {
        use super::*;
        struct OneShotWithAttribsObserver {
            // holds sender
        }
        impl OneShotWithAttribsObserver {
            fn one_shot_with_attribs(&self, attributes: OneShotWithAttribs) {
                // emits event
                todo!()
            }
        }
    }

    mod analyzer {
        use super::*;
        // Still single event entity. If it ever arrived, we know it in its
        // entirety.
        trait SewaModel: quent_analyzer::Entity {
            fn sewa() -> Sewa;
        }
    }
}

mod multi_one_shot {
    use super::*;

    mod model {
        use super::*;

        struct X {
            foo: u64,
        }

        struct Y {
            bar: String
        }

        #[derive(Entity)]
        struct MultiOneShot {
            a: Once<X>,
            b: Once<Y>,
            c: Once<Y>, // same attributes type, but semantically different event
            d: Once<()>
        }
    }

    mod events {
        use super::*;
    }

    mod instrumentation {
        use super::*;
    }

    mod analyzer {
        use super::*;
    }
}

mod one_multi_shot {
    use super::*;

    mod model {
        use super::*;

        struct X {
            foo: u64,
        }

        #[derive(Entity)]
        struct OneMultiShot {
            a: Multi<X>,
        }
    }

    mod events {
        use super::*;
    }

    mod instrumentation {
        use super::*;
    }

    mod analyzer {
        use super::*;
    }
}

mod multi_multi_shot {
    use super::*;

    mod model {
        use super::*;

        struct X {
            foo: u64,
        }

        struct Y {
            bar: String
        }

        #[derive(Entity)]
        struct MultiMulti {
            a: Multi<X>,
            b: Multi<X>,
            c: Multi<Y>
        }
    }

    mod events {
        use super::*;
    }

    mod instrumentation {
        use super::*;
    }

    mod analyzer {
        use super::*;
    }
}
