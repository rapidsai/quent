use py_rs::PY;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../entities");

    // Export TypeScript bindings to ts-bindings directory
    <quent_entities::engine::Engine as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::query_group::QueryGroup as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::worker::Worker as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::query::Query as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::timeline::ResourceTimeline as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinned as TS>::export_all_to("./ts-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinnedByState as TS>::export_all_to(
        "./ts-bindings/",
    )?;
    <quent_analyzer::query::QueryBundle as TS>::export_all_to("./ts-bindings/")?;

    // Export Python bindings to py-bindings directory
    <quent_entities::engine::Engine as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::query_group::QueryGroup as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::worker::Worker as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::query::Query as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::timeline::ResourceTimeline as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinned as PY>::export_all_to("./py-bindings/")?;
    <quent_entities::timeline::ResourceTimelineBinnedByState as PY>::export_all_to(
        "./py-bindings/",
    )?;
    <quent_analyzer::query::QueryBundle as PY>::export_all_to("./py-bindings/")?;

    Ok(())
}
