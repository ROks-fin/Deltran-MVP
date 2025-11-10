fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable SQLx offline mode to skip compile-time query verification
    // This allows building without database connectivity
    println!("cargo:rustc-env=SQLX_OFFLINE=true");
    println!("cargo:rerun-if-changed=proto/settlement.proto");
    println!("cargo:rerun-if-changed=proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/settlement.proto"], &["proto"])?;
    Ok(())
}
