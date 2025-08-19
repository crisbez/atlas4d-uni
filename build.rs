
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/atlas4d.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/atlas4d.proto");
    Ok(())
}
