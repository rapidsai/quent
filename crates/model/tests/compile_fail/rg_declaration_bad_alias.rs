// Resource group declaration alias must match an event.
#[derive(quent_model::Attributes, serde::Serialize, serde::Deserialize)]
pub struct MyEvent {}

quent_model::entity! {
    Bad: ResourceGroup {
        events: {
            a: MyEvent,
        },
        declaration: nonexistent,
    }
}

fn main() {}
