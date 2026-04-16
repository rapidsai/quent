// Resource group with events requires `declaration:` keyword.
#[derive(quent_model::Attributes, serde::Serialize, serde::Deserialize)]
pub struct MyEvent {}

quent_model::entity! {
    Bad: ResourceGroup {
        events: {
            a: MyEvent,
        },
    }
}

fn main() {}
