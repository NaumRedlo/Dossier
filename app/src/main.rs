//! The Dossier render client.
//!
//! The terminal worker it replaces is `client/` — five thousand lines of Python
//! that claims a job, fetches what it needs, runs the engine and sends the
//! video back. This grows into the same thing with a window in front of it, and
//! with the engine linked in rather than shelled out to.
//!
//! Windows without a console: `windows_subsystem = "windows"` in a release
//! build, or the application opens with a black terminal behind it, which is
//! the thing it exists to get away from.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod check;

/// The readiness list, for the screen that replaces `--check`.
#[tauri::command]
fn ready() -> Vec<check::Row> {
    check::ready()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ready])
        .run(tauri::generate_context!())
        .expect("the window could not be opened");
}
