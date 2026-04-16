// fsm! transition target must exist in states list.
quent_model::state! {
    Idle {}
}

quent_model::fsm! {
    Bad {
        states: { idle: Idle },
        entry: idle,
        exit_from: { idle },
        transitions: { idle => ghost },
    }
}

fn main() {}
