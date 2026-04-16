// model! rejects duplicate component names.
quent_model::entity! {
    Root: ResourceGroup<Root = true> {}
}

quent_model::entity! {
    Thing {
        attributes: { x: u64 },
    }
}

quent_model::model! {
    App {
        root: Root,
        Thing,
        Thing,
    }
}

fn main() {}
