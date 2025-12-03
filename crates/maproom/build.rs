fn main() {
    // Ensure the vendor directory exists
    let vendor_dir = std::path::Path::new("vendor/sqlite-vec");
    if vendor_dir.exists() {
        cc::Build::new()
            .file("vendor/sqlite-vec/sqlite-vec.c")
            .compile("sqlite_vec");

        println!("cargo:rustc-link-lib=static=sqlite_vec");
    } else {
        println!(
            "cargo:warning=sqlite-vec source not found in vendor/sqlite-vec. Skipping compilation."
        );
    }
}
