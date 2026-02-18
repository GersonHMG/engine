// proto.rs — Include protobuf-codegen generated protobuf types

// protobuf-codegen generates one .rs file per .proto file.
// We include them all and re-export everything.

#[allow(clippy::all)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}
