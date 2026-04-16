// declaration: is only valid on resource group entities.
#[derive(quent_model::Attributes, serde::Serialize, serde::Deserialize)]
pub struct MyEvent {}

quent_model::entity! {
    Bad {
        events: {
            a: MyEvent,
        },
        declaration: a,
    }
}

fn main() {}
