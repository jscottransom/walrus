fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate protobuf code
    prost_build::compile_protos(&["src/api/v1/log.proto"], &["src/api/v1"])?;
    
    // Generate tonic code
    tonic_build::compile_protos("src/api/v1/log.proto")?;

    Ok(())
}
