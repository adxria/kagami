fn main() {
    // Recompile Windows resources when the bundled icons change.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/test.ico");
    tauri_build::build()
}
