fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/quent/v1/quent/collector.proto")?;
    Ok(())
}
