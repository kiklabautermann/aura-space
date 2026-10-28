fn main() {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        println!("cargo:rustc-link-search=native={}/.cargo/lib", manifest_dir);
    }
}
