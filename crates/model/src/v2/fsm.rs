use crate::v2::entity::{EntityHandle, ObserverError};
use quent_model_macros::Fsm;
use std::marker::PhantomData;
use uuid::Uuid;

// Considerations:
//
// While theoretically we could first generate a #[derive(Entity)] from a
// #[derive(Fsm)], it would be harder to generate FSM entity instrumentation
// APIs with the type-state pattern from there, so #[derive(Fsm)] will not take
// that approach, but we should figure out what functionaltiy between those two
// derives overlaps and deduplicate any logic.
//
// To compile a set of states an Fsm can be in, I've considered declaring a
// struct where each field is the state name and the field type are the
// attribute types. However, I find the enum style more compelling since an FSM
// is always in exactly one state at any moment, which naturally translates to a
// sum type.

// None of these should compile. States need a name and an attributes type,
// which none of these definitions can provide:
mod invalid {
    use super::*;

    mod model {
        use super::*;

        #[derive(Fsm)]
        pub enum Invalid {}
    }
}

// FSM with just one state without attributes
mod single_empty {
    use super::*;

    mod model {
        use super::*;

        // No quent(transition) macro attribute. Since there is one state, it
        // must be the entry state and it is a final state.
        #[derive(Fsm)]
        pub enum SingleEmpty {
            A,
        }
    }
}

// FSM with just one state with atttributes
mod single_attribs {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        // Same here, no quent(transition) macro attribute. The only state is A,
        // which is implicitly the entry state and a final state.
        #[derive(Fsm)]
        pub enum SingleAttribs {
            A(X),
        }
    }
}

// FSM with multiple states through which it must go in a sequence
mod multi_seq {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            pub foo: u64,
        }

        pub struct Y {
            pub foo: u64,
            pub bar: String,
        }

        #[derive(Fsm)]
        #[quent(transitions={entry->A, A->B, B->C, C->exit})]
        pub enum MultiSeq {
            A(X),
            B(Y),
            C(Y), // same attributes type, but semantically different state
        }

        // Note: we could provide syntactic sugar later to simplify this to:
        // #[quent(transitions={entry->A->B->C->exit})]
    }

    mod events {
        // No new event type, because the FSM enum is already the event payload
        // type
    }

    mod instrumentation {
        use super::*;

        // Types generated to support the type-state pattern below
        pub struct A;
        pub struct B;
        pub struct C;

        pub struct MultiSeqObserver {
            // holds same stuff as in entity examples
        }

        impl MultiSeqObserver {
            // Initial state transition produces a handle with an API following
            // the type-state pattern
            pub fn a(
                &self,
                _attributes: super::model::X,
            ) -> Result<MultiSeqHandle<A>, ObserverError> {
                todo!()
            }
        }

        // A handle for the FSM
        pub struct MultiSeqHandle<T> {
            _phantom: PhantomData<T>,
            id: Uuid,
        }

        impl<T> EntityHandle for MultiSeqHandle<T> {
            fn id(&self) -> Uuid {
                todo!()
            }
        }

        impl MultiSeqHandle<A> {
            pub fn b(
                self,
                _attributes: super::model::Y,
            ) -> Result<MultiSeqHandle<B>, ObserverError> {
                todo!()
            }
        }

        impl MultiSeqHandle<B> {
            pub fn c(
                self,
                _attributes: super::model::Y,
            ) -> Result<MultiSeqHandle<C>, ObserverError> {
                todo!()
            }
        }

        impl MultiSeqHandle<C> {
            pub fn exit(self) -> Result<MultiSeqHandle<()>, ObserverError> {
                todo!()
            }
        }
    }

    mod usage {
        use crate::v2::entity::EntityHandle;

        fn example() -> Result<(), Box<dyn std::error::Error>> {
            let obs = super::instrumentation::MultiSeqObserver {};

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
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        #[derive(Fsm)]
        #[quent(transitions = {entry->A, A->A, A->exit})]
        pub enum Multi {
            A(X),
        }
    }
}

// FSM with a state with multiple next states
mod fanout {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Fsm)]
        #[quent(transitions={entry->A, A->{B, C}, B->D, C->D, D->exit})]
        pub enum Multi {
            A(X),
            B,
            C(Y),
            D,
        }
    }
}

mod fanin {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Fsm)]
        #[quent(transitions={entry->A, A->{B,C}, {B, C}->D, D->exit})]
        pub enum Multi {
            A(()),
            B(Y),
            C,
            D(X),
        }
    }
}

mod full {
    use super::*;

    mod model {
        use super::*;

        pub struct X {
            foo: u64,
        }

        pub struct Y {
            bar: String,
        }

        #[derive(Fsm)]
        #[quent(transitions={entry->A, A->{B,C}, B->B, {B, C}->D, D->exit})]
        pub enum Multi {
            A(()),
            B(Y),
            C,
            D(X),
        }
    }
}
