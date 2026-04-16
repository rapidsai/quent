// entity! cannot have both attributes and events on a non-resource-group.
quent_model::entity! {
    Bad {
        attributes: {
            x: u64,
        },
        events: {
            a: (),
        },
    }
}

fn main() {}
