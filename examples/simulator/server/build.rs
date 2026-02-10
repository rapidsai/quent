use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Export TypeScript bindings to ts-bindings directory
    <quent_simulator_ui::QueryBundle as TS>::export_all_to("./ts-bindings/")?;

    <quent_simulator_ui::ResourceTimelineUrlQueryParams as TS>::export_all_to("./ts-bindings/")?;
    <quent_simulator_ui::TimelineResponse as TS>::export_all_to("./ts-bindings/")?;

    Ok(())
}
