fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["src/api/v1/log.proto"], &["src/api/v1/log"])?;
    // tonic_build::compile_protos("src/api/v1/log.proto")?;

    Ok(())
}
