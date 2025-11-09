fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable SQLx offline mode for compilation without database
    // Note: Run `cargo sqlx prepare` when database schema is ready to generate query cache
    println!("cargo:rustc-env=SQLX_OFFLINE=true");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/settlement.proto"], &["proto"])?;
    Ok(())
}
