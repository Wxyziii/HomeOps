mod redux_bridge;

pub fn run() {
    let builder = tauri::Builder::default();
    // H2.1 — local Redux Maker bridge commands (desktop-only). The web/browser
    // build never reaches these; the Ubuntu server-agent has none of them.
    let builder = redux_bridge::register(builder);
    builder
        .run(tauri::generate_context!())
        .expect("error while running HomeOps Panel");
}
