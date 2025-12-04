use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../entities");
    quent_entities::engine::Engine::export_all()?;
    Ok(())
}
