fn main() {
    // Enable SQLx offline mode to skip compile-time query verification
    // This allows building without database connectivity
    println!("cargo:rustc-env=SQLX_OFFLINE=true");
}
