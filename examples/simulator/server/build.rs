use quent_simulator_ui::{
    QueryBundle,
    timeline::{
        BulkTimelinesRequest, BulkTimelinesResponse, ResourceTimelineUrlQueryParams,
        TimelineResponse,
    },
};
use ts_rs::TS;

const TS_OUT_DIR: &str = "./ts-bindings/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Export TypeScript bindings to ts-bindings directory
    <QueryBundle as TS>::export_all_to(TS_OUT_DIR)?;

    <ResourceTimelineUrlQueryParams as TS>::export_all_to(TS_OUT_DIR)?;
    <TimelineResponse as TS>::export_all_to(TS_OUT_DIR)?;
    <BulkTimelinesRequest as TS>::export_all_to(TS_OUT_DIR)?;
    <BulkTimelinesResponse as TS>::export_all_to(TS_OUT_DIR)?;

    Ok(())
}
