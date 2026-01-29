use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../entities");

    // Export TypeScript bindings to ts-bindings directory
    <quent_entities::timeline::TimelineResponse as TS>::export_all_to("./ts-bindings/")?;
    <quent_analyzer::query_bundle::QueryBundle as TS>::export_all_to("./ts-bindings/")?;

    Ok(())
}
