//! WASM entry point (built by Trunk). Shared by the web build and the desktop webview.

#[cfg(test)]
mod boot_tests;
mod logger;

fn main() {
    console_error_panic_hook::set_once();
    // App records at Info; the Cubism framework channel Off, like V1 (L2D-015).
    logger::init();
    leptos::mount::mount_to_body(amadeus_ui::App);
}
