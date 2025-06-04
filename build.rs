fn main() {
    prost_build::compile_protos(&["src/api/v1/log.proto"], &["src/api/v1/log"]).unwrap();
}
