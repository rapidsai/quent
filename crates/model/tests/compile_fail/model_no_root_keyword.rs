// model! first entry must be `root: <Type>`.
quent_model::entity! {
    Cluster: ResourceGroup<Root = true> {}
}

quent_model::model! {
    App {
        Cluster,
    }
}

fn main() {}
