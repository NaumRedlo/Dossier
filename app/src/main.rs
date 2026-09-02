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

mod bench;
mod bot;
mod check;
mod draw;
mod machine;
mod work;

/// Draw a replay. Blocking on purpose — Tauri runs a command off the main
/// thread, and a render is the one thing this application exists to do, so
/// there is nothing else for that thread to be doing meanwhile.
#[tauri::command]
fn draw(replay: String, songs: String, out: String) -> Result<Drawn, String> {
    let told = draw::Told::default();
    let asked = draw::Asked {
        replay: std::path::Path::new(&replay),
        songs: Some(std::path::Path::new(&songs)),
        map: None,
        out: std::path::PathBuf::from(out),
        size: (1920, 1080),
        fps: 60.0,
        from_ms: None,
        to_ms: None,
        background: true,
        storyboard: true,
        bare: false,
        mute: false,
        skin: None,
        events: false,
    };
    draw::draw(&asked, &told).map(|path| Drawn {
        path: path.display().to_string(),
        said: told.said(),
    })
}

/// Where the file went, and everything the pipeline said on the way.
#[derive(serde::Serialize)]
struct Drawn {
    path: String,
    said: Vec<String>,
}

/// One turn of the worker's loop: ask for a job, and do it if there is one.
///
/// One turn rather than a loop, so that the window decides when to ask again —
/// and so that nothing runs on somebody's machine that they did not press.
#[tauri::command]
fn work_once(server: String, token: String, name: String, songs: String) -> Result<String, String> {
    let bot = bot::Bot::new(&server, &token, &name).map_err(|e| e.to_string())?;
    let along = std::sync::Arc::new(std::sync::Mutex::new(work::Along::default()));
    let engine = format!("dossier {}", env!("CARGO_PKG_VERSION"));
    match work::once(
        &bot,
        &engine,
        &machine::capacity(),
        std::path::Path::new(&songs),
        &along,
    ) {
        Ok(work::Did::Nothing) => Ok("нечего делать".to_owned()),
        Ok(work::Did::Delivered { title }) => Ok(format!("готово: {title}")),
        Ok(work::Did::GaveBack { why }) => Ok(format!("вернул задачу: {why}")),
        Err(refused) => Err(refused.to_string()),
    }
}

/// What this machine is, what it will give, and how fast it draws.
#[tauri::command]
fn profile() -> machine::Profile {
    machine::profile()
}

/// The readiness list, for the screen that replaces `--check`.
#[tauri::command]
fn ready() -> Vec<check::Row> {
    check::ready()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ready, draw, work_once, profile])
        .run(tauri::generate_context!())
        .expect("the window could not be opened");
}
