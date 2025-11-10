fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Disable SQLx offline mode to allow compile-time query verification against live database
    // Reference: https://github.com/launchbadge/sqlx/blob/main/FAQ.md
    println!("cargo:rustc-env=SQLX_OFFLINE=false");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/settlement.proto"], &["proto"])?;
    Ok(())
}
