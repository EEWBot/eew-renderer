use std::io::Result;

fn main() -> Result<()> {
    println!("cargo::rerun-if-changed=../assets/eew-renderer-proto/net.eewbot.proto");

    std::fs::copy(
        "../assets/eew-renderer-proto/net.eewbot.proto",
        "src/net.eewbot.proto",
    )?;

    prost_build::compile_protos(&["src/net.eewbot.proto"], &["src"])?;

    Ok(())
}
