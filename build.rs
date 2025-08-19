fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Използвай вграден protoc, за да не изисква системен пакет
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/atlas4d.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/atlas4d.proto");
    Ok(())
}
