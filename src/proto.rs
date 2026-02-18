// proto.rs — Include prost-generated protobuf types

// prost-build generates all messages without a package name into `_.rs`.
// We include that file and re-export everything.

#[allow(clippy::all)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}
