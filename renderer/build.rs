use std::path::PathBuf;

const PROTO_OUTPUT_DIRECTORY: &str = "src/protos/";

fn main() {
    let generate_to = PathBuf::from(PROTO_OUTPUT_DIRECTORY);
    if !generate_to.exists() {
        std::fs::create_dir_all(generate_to).expect("Failed to create target directory");
    }

    protobuf_codegen::Codegen::new()
        .pure()
        .customize(protobuf_codegen::Customize::default().gen_mod_rs(true))
        .out_dir(PROTO_OUTPUT_DIRECTORY)
        .include("../assets/vector-tile-proto/")
        .input("../assets/vector-tile-proto/vector_tile.proto")
        .run()
        .expect("Failed to generate code from proto file");
}
