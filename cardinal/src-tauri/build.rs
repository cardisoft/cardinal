fn main() {
    println!("cargo:rustc-link-lib=framework=QuickLook");

    tauri_build::build()
}
