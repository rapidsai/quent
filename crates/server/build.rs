use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../entities");

    // Export TypeScript bindings to ts-bindings directory
    <quent_entities::engine::Engine as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::query_group::QueryGroup as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::worker::Worker as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::query::Query as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinned as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinnedByState as TS>::export_all_to(
        "./ts-bindings/",
    )?;
    <quent_analyzer::query_bundle::QueryBundle as TS>::export_all_to("./ts-bindings/")?;

    Ok(())
}
