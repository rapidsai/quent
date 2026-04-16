// fsm! exit_from state must exist in states list.
quent_model::state! {
    Idle {}
}

quent_model::fsm! {
    Bad {
        states: { idle: Idle },
        entry: idle,
        exit_from: { nonexistent },
        transitions: {},
    }
}

fn main() {}
