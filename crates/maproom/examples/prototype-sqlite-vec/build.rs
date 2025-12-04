use std::path::Path;

fn main() {
    let vendor_dir = Path::new("vendor/sqlite-vec");

    if !vendor_dir.exists() {
        println!("cargo:warning=sqlite-vec source not found in vendor/sqlite-vec. Assuming manual setup or skipping.");
        // In a real build, we'd probably panic or download it here.
        // For this prototype, we'll assume we download it manually as part of the ticket steps.
    }

    cc::Build::new()
        .file("vendor/sqlite-vec/sqlite-vec.c")
        .compile("sqlite_vec");

    println!("cargo:rustc-link-lib=static=sqlite_vec");
}
