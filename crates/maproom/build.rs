fn main() {
    // Ensure the vendor directory exists
    let vendor_dir = std::path::Path::new("vendor/sqlite-vec");
    if vendor_dir.exists() {
        // Vendored sqlite-vec version: v0.1.3
        // Source: https://github.com/asg017/sqlite-vec/releases/tag/v0.1.3
        // Commit: 496560cf9ac4b358ea43793e591f376c02c16b90
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
