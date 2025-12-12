use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../entities");
    quent_entities::engine::Engine::export_all()?;
    quent_entities::query_group::QueryGroup::export_all()?;
    quent_entities::worker::Worker::export_all()?;
    quent_entities::query::Query::export_all()?;
    quent_events::attributes::Attribute::export_all()?;
    Ok(())
}
