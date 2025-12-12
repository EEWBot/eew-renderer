use std::io::Result;

fn main() -> Result<()> {
    std::fs::copy(
        "../assets/eew-renderer-proto/quake_prefecture_v0.proto",
        "src/quake_prefecture_v0.proto",
    )?;
    std::fs::copy(
        "../assets/eew-renderer-proto/tsunami_v0.proto",
        "src/tsunami_v0.proto",
    )?;

    prost_build::compile_protos(
        &[
            "src/quake_prefecture_v0.proto",
            "src/tsunami_v0.proto"
        ],
        &["src"],
    )?;

    Ok(())
}
