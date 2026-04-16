// fsm! transition source must exist in states list.
quent_model::state! {
    Idle {}
}

quent_model::fsm! {
    Bad {
        states: { idle: Idle },
        entry: idle,
        exit_from: { idle },
        transitions: { ghost => idle },
    }
}

fn main() {}
