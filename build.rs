// build.rs — Compile .proto files with protobuf-codegen (pure Rust, no protoc needed)

fn main() {
    protobuf_codegen::Codegen::new()
        .pure()
        .includes(&["src/protobuf/protos/"])
        .inputs(&[
            "src/protobuf/protos/ssl_vision_wrapper.proto",
            "src/protobuf/protos/ssl_vision_detection.proto",
            "src/protobuf/protos/ssl_vision_geometry.proto",
            "src/protobuf/protos/ssl_gc_referee_message.proto",
            "src/protobuf/protos/ssl_gc_common.proto",
            "src/protobuf/protos/ssl_gc_geometry.proto",
            "src/protobuf/protos/ssl_gc_game_event.proto",
            "src/protobuf/protos/ssl_gc_state.proto",
            "src/protobuf/protos/ssl_gc_change.proto",
            "src/protobuf/protos/ssl_gc_engine.proto",
            "src/protobuf/protos/ssl_gc_engine_config.proto",
            "src/protobuf/protos/ssl_gc_api.proto",
            "src/protobuf/protos/ssl_gc_rcon.proto",
            "src/protobuf/protos/ssl_gc_rcon_autoref.proto",
            "src/protobuf/protos/ssl_gc_rcon_remotecontrol.proto",
            "src/protobuf/protos/ssl_gc_rcon_team.proto",
            "src/protobuf/protos/ssl_simulation_config.proto",
            "src/protobuf/protos/ssl_simulation_control.proto",
            "src/protobuf/protos/ssl_simulation_error.proto",
            "src/protobuf/protos/ssl_simulation_robot_control.proto",
            "src/protobuf/protos/ssl_simulation_robot_feedback.proto",
            "src/protobuf/protos/ssl_simulation_synchronous.proto",
            "src/protobuf/protos/grSim_Commands.proto",
            "src/protobuf/protos/grSim_Packet.proto",
            "src/protobuf/protos/grSim_Replacement.proto",
            "src/protobuf/protos/grSim_Robotstatus.proto",
        ])
        .cargo_out_dir("protos")
        .run_from_script();

    tauri_build::build();
}
