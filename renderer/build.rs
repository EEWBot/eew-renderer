use std::io::Result;


fn main() -> Result<()> {
    std::fs::copy(
        "../assets/eew-renderer-proto/quake_prefecture_v0.proto",
        "src/quake_prefecture_v0.proto"
    )?;

    prost_build::compile_protos(&["src/quake_prefecture_v0.proto"], &["src"])?;

    Ok(())
}
