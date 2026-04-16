// fsm! entry state must exist in states list.
quent_model::state! {
    Idle {}
}

quent_model::fsm! {
    Bad {
        states: { idle: Idle },
        entry: nonexistent,
        exit_from: { idle },
        transitions: {},
    }
}

fn main() {}
