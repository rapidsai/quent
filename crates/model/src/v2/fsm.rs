use crate::v2::entity::{EntityHandle, ObserverError};
use std::marker::PhantomData;
use uuid::Uuid;

// Considerations:
//
// While theoretically we could first generate a #[derive(Entity)] from a
// #[derive(Fsm)], it would be harder to generate FSM entity instrumentation
// APIs with the type-state pattern from there, so #[derive(Fsm)] will not take
// that approach, but we should figure out what functionaltiy between those two
// derives overlaps and deduplicate any logic.

// Note on alternatives:
//
// - Alternative 0: FSMs are declared as structs. The fields represent state
// name: attribute type.
//
// - Alternative 1: FSMs are declared as enums. Each enum variant represents
// the state name(attribute type).

// None of these should compile. States need a name and an attributes type,
// which none of these definitions can provide:
mod invalid {
    mod model {
        // Alternative 0:
        #[derive(Fsm)]
        pub struct Invalid0_1;

        #[derive(Fsm)]
        pub struct Invalid0_2 {}

        #[derive(Fsm)]
        pub struct Invalid0_3();

        #[derive(Fsm)]
        pub struct Invalid0_4(());

        // Alternative 1:
        #[derive(Fsm)]
        pub enum Invalid1 {}
    }
}

// FSM with just one state without attributes
mod single_empty {

    mod model {
        #[derive(Fsm)]
        pub struct SingleEmpty0 {
            // A struct field is a state the FSM can transition into. The field
            // attribute marks this as the initial state and a state from which
            // we can exit.
            #[quent(entry, exit)]
            a: (),
        }

        pub enum SingleEmpty1 {
            #[quent(entry, exit)]
            A,
        }
    }
}

// FSM with just one state with atttributes
mod single_attribs {

    mod model {

        pub struct X {
            foo: u64,
        }

        #[derive(Fsm)]
        pub struct SingleAttribs0 {
            #[quent(entry, exit)]
            a: X,
        }

        #[derive(Fsm)]
        pub enum SingleAttribs1 {
            #[quent(entry, exit)]
            A,
        }
    }
}

// FSM with multiple states through which it must go in a sequence
mod multi_seq {
    use super::*;

    mod model {

        pub struct X {
            pub foo: u64,
        }

        pub struct Y {
            pub foo: u64,
            pub bar: String,
        }

        // Alternative 0
        #[derive(Fsm)]
        pub struct Multi_0 {
            #[quent(entry, to=b)]
            a: X,
            #[quent(to=c)]
            b: Y,
            #[quent(exit)]
            c: Y, // same attributes type, but semantically different state
        }

        // Alternative 1
        #[derive(Fsm)]
        pub enum Multi_1 {
            #[quent(entry, to=b)]
            A(X),
            #[quent(to=c)]
            B(Y),
            #[quent(exit)]
            C(Y), // same attributes type, but semantically different state
        }
    }

    mod events {
        // Note the benefit of alternative 1 is that this definition is the
        // same.
        pub enum MultiEvents {
            A(super::model::X),
            B(super::model::Y),
            C(super::model::Y),
        }
    }

    mod instrumentation {
        use super::*;

        // Types generated to support the type-state pattern below
        pub struct A;
        pub struct B;
        pub struct C;

        pub struct MultiObserver {
            // holds same stuff as in entity examples
        }

        impl MultiObserver {
            // Initial state transition produces a handle with an API following the type-state pattern
            pub fn a(&self, attributes: super::model::X) -> Result<MultiHandle<A>, ObserverError> {
                todo!()
            }
        }

        // A handle for the FSM
        pub struct MultiHandle<T> {
            _phantom: PhantomData<T>,
            id: Uuid,
        }

        impl<T> EntityHandle for MultiHandle<T> {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl MultiHandle<A> {
            pub fn b(self, _attributes: super::model::Y) -> Result<MultiHandle<B>, ObserverError> {
                todo!()
            }
        }

        impl MultiHandle<B> {
            pub fn c(self, _attributes: super::model::Y) -> Result<MultiHandle<C>, ObserverError> {
                todo!()
            }
        }

        impl MultiHandle<C> {
            pub fn exit(self) -> Result<MultiHandle<()>, ObserverError> {
                todo!()
            }
        }
    }

    mod usage {
        use crate::v2::entity::EntityHandle;

        fn example() -> Result<(), Box<dyn std::error::Error>> {
            let obs = super::instrumentation::MultiObserver {};

            let handle = obs.a(super::model::X { foo: 1337 })?;

            // handle.c() - doesn't compile

            let handle = handle.b(super::model::Y {
                foo: 1338,
                bar: "hi".to_string(),
            })?;

            let handle = handle
                .c(super::model::Y {
                    foo: 1339,
                    bar: "bbye".to_string(),
                })?
                .exit()?; // <-- we can chain if we want

            // can't call any more transitions on handle after exit
            // can still get the id if we want:
            println!("{}", handle.id());

            Ok(())
        }
    }
}

// FSM with a single state that can transition into itself
mod solo_loop {
    mod model {
        pub struct X {
            foo: u64,
        }

        #[derive(Fsm)]
        pub struct Multi {
            #[quent(entry, to=a, exit)]
            a: X,
        }
    }
}
