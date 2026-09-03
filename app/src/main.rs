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
mod library;
mod link;
mod machine;
mod settings;
mod work;

/// Draw a replay of this person's own, and say how it is going while it does.
///
/// Blocking on purpose — Tauri runs a command off the main thread, so the
/// window stays alive — but a render is minutes, and a window that says nothing
/// for minutes is one somebody force-quits. The engine's own events are
/// forwarded as they arrive; see `dossier_produce::events`.
#[tauri::command]
fn draw(
    app: tauri::AppHandle,
    replay: String,
    out: String,
    skin: Option<String>,
) -> Result<Drawn, String> {
    use tauri::Emitter;

    let said = settings::Settings::load();
    let told = draw::Told::default();
    let sending = app.clone();
    dossier_produce::events::listen(move |line| {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
            let _ = sending.emit("drawing", event);
        }
    });
    // A name from the shelf, not the shelf itself: `from_folder` reads one
    // skin, and handing it a folder of them reads whatever is nearest.
    let skin = skin
        .filter(|name| !name.is_empty())
        .map(|name| std::path::Path::new(&said.skins).join(name))
        .filter(|path| path.is_dir());
    let asked = draw::Asked {
        replay: std::path::Path::new(&replay),
        songs: Some(std::path::Path::new(&said.songs)),
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
        skin,
        events: true,
    };
    let done = draw::draw(&asked, &told);
    dossier_produce::events::unlisten();
    done.map(|path| Drawn {
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

/// What this machine has been told, and whether it has been told anything.
#[tauri::command]
fn settings_read() -> settings::Settings {
    settings::Settings::load()
}

/// Whether the setup wizard should open instead of the ordinary window.
#[tauri::command]
fn first_run() -> bool {
    settings::Settings::load().first_run()
}

#[tauri::command]
fn settings_write(said: settings::Settings) -> Result<(), String> {
    said.save()
}

/// The skins on the shelf, by name, for a picker.
#[tauri::command]
fn skins() -> Vec<String> {
    library::skins(&settings::Settings::load())
}

/// What is on this machine's shelves.
#[tauri::command]
fn shelves() -> library::Library {
    library::look(&settings::Settings::load())
}

/// The replays this person could ask to have drawn, newest first.
#[tauri::command]
fn my_replays(most: Option<usize>) -> Vec<library::Played> {
    library::played(&settings::Settings::load(), most.unwrap_or(60))
}

/// Everybody else on the farm.
#[tauri::command]
fn farm() -> Result<bot::Farm, String> {
    let said = settings::Settings::load();
    bot::Bot::new(&said.server, &said.token, &said.name)
        .and_then(|bot| bot.farm())
        .map_err(|refused| refused.to_string())
}

/// The readiness list, for the screen that replaces `--check`.
#[tauri::command]
fn ready() -> Vec<check::Row> {
    check::ready()
}

/// Hand a link to the system: the repository, a mail draft, a new issue.
///
/// Nothing is ever sent from here. The most this does is open somebody's mail
/// client with the letter already written — whether it goes is their key.
#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    link::open(&url)
}

/// The three lines worth putting at the bottom of a bug report, so that nobody
/// has to be asked for them. What this is, and what it is running on — no
/// paths, no token, no name.
#[tauri::command]
fn about() -> About {
    let hardware = machine::Hardware::read();
    About {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        os: hardware.os,
        cpu: hardware.cpu,
        cores: hardware.cores,
    }
}

#[derive(serde::Serialize)]
struct About {
    version: String,
    os: String,
    cpu: String,
    cores: u32,
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ready,
            draw,
            work_once,
            profile,
            settings_read,
            settings_write,
            first_run,
            shelves,
            my_replays,
            skins,
            farm,
            open_link,
            about
        ])
        .run(tauri::generate_context!())
        .expect("the window could not be opened");
}
