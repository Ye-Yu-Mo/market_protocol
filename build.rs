use std::env;
use std::path::PathBuf;

const PROTO_PATH: &str = "proto/market_protocol/v1/market_protocol.proto";
const INCLUDE_ROOT: &str = "proto";

fn main() {
    println!("cargo:rerun-if-changed={PROTO_PATH}");
    println!("cargo:rerun-if-changed=build.rs");

    let protoc = protoc_bin_vendored::protoc_bin_path().unwrap_or_else(|error| {
        panic!("failed to locate vendored protoc for {PROTO_PATH}: {error}")
    });
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("Cargo did not provide OUT_DIR for protobuf generation"),
    );

    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir).protoc_executable(protoc);
    config
        .compile_protos(&[PROTO_PATH], &[INCLUDE_ROOT])
        .unwrap_or_else(|error| panic!("failed to compile {PROTO_PATH}: {error}"));
}
