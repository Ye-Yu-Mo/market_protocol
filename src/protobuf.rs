//! Rust types generated directly from the canonical Protobuf schema.

pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/market_protocol.v1.rs"));
}
