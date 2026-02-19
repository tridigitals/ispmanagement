fn main() {
    // Silence `unexpected cfg condition name: mobile` warnings from Tauri templates.
    println!("cargo::rustc-check-cfg=cfg(mobile)");

    // Allow building a backend-only server binary without pulling Tauri/WebView dependencies.
    // Enable via `--features desktop` (default).
    #[cfg(feature = "desktop")]
    tauri_build::build();
}
