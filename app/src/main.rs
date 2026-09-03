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
mod look;
mod machine;
mod pick;
mod pics;
mod play;
mod reel;
mod settings;
mod update;
mod work;

/// Draw a replay of this person's own, and say how it is going while it does.
///
/// Blocking on purpose — Tauri runs a command off the main thread, so the
/// window stays alive — but a render is minutes, and a window that says nothing
/// for minutes is one somebody force-quits. The engine's own events are
/// forwarded as they arrive; see `dossier_produce::events`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn draw(
    app: tauri::AppHandle,
    replay: String,
    out: String,
    skin: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    background: Option<bool>,
    storyboard: Option<bool>,
    mute: Option<bool>,
    from_ms: Option<f64>,
    to_ms: Option<f64>,
    fine: Option<draw::Fine>,
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
        size: (width.unwrap_or(1920), height.unwrap_or(1080)),
        fps: fps.unwrap_or(60.0),
        from_ms,
        to_ms,
        background: background.unwrap_or(true),
        storyboard: storyboard.unwrap_or(true),
        bare: false,
        mute: mute.unwrap_or(false),
        skin,
        events: true,
        fine: fine.unwrap_or_default(),
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

/// Judge a replay and hand back what became of every object.
///
/// No frames, no ffmpeg, no waiting: the engine already knows what each click
/// was worth, and a play can be looked at in the time it takes to read the
/// file. This is what the viewer draws its strip from.
#[tauri::command]
fn judged(replay: String) -> Result<play::Scene, String> {
    let said = settings::Settings::load();
    let songs = std::path::PathBuf::from(&said.songs);
    let songs = songs.is_dir().then_some(songs.as_path());
    play::open(std::path::Path::new(&replay), songs)
}

/// Ask the system for a folder. `None` means the dialog was closed.
#[tauri::command]
fn pick_folder(prompt: String) -> Result<Option<String>, String> {
    pick::folder(&prompt)
}

/// Put an `.osk` on the shelf. Returns the name it went under.
#[tauri::command]
fn install_skin(path: String) -> Result<String, String> {
    library::install_skin(&settings::Settings::load(), std::path::Path::new(&path))
}

/// The pictures the viewer draws a play with, out of a named skin.
///
/// Empty name means the one settings call the default, which is the answer to
/// "покажи со скином" — the skin this application is set to.
#[tauri::command]
fn skin_pictures(name: Option<String>) -> pics::Pictures {
    let said = settings::Settings::load();
    let name = name.filter(|n| !n.is_empty()).unwrap_or(said.skin.clone());
    if name.is_empty() || said.skins.is_empty() {
        return pics::Pictures::default();
    }
    let folder = std::path::Path::new(&said.skins).join(name);
    if folder.is_dir() {
        pics::of(&folder)
    } else {
        pics::Pictures::default()
    }
}

/// Ask the system for one skin archive.
#[tauri::command]
fn pick_skin(prompt: String) -> Result<Option<String>, String> {
    pick::file(&prompt, "osk")
}

/// Ask the system for one replay file.
#[tauri::command]
fn pick_replay(prompt: String) -> Result<Option<String>, String> {
    pick::file(&prompt, "osr")
}

/// Draw the spans somebody cut on the timeline, and join them into one file.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn build_reel(
    app: tauri::AppHandle,
    replay: String,
    out: String,
    spans: Vec<reel::Span>,
    skin: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    mute: Option<bool>,
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
    let skin = skin
        .filter(|name| !name.is_empty())
        .map(|name| std::path::Path::new(&said.skins).join(name))
        .filter(|path| path.is_dir());
    let asked = draw::Asked {
        replay: std::path::Path::new(&replay),
        songs: Some(std::path::Path::new(&said.songs)),
        map: None,
        out: std::path::PathBuf::from(out),
        size: (width.unwrap_or(1920), height.unwrap_or(1080)),
        fps: fps.unwrap_or(60.0),
        from_ms: None,
        to_ms: None,
        background: true,
        storyboard: true,
        bare: false,
        mute: mute.unwrap_or(false),
        skin,
        events: true,
        fine: draw::Fine::default(),
    };
    let counting = app.clone();
    let step = move |done: usize, of: usize| {
        let _ = counting.emit("reeling", serde_json::json!({ "clip": done + 1, "of": of }));
    };
    let done = reel::build(&asked, &spans, &told, &step);
    dossier_produce::events::unlisten();
    done.map(|path| Drawn {
        path: path.display().to_string(),
        said: told.said(),
    })
}

/// Is there a newer Dossier, and what would updating mean on this machine.
#[tauri::command]
fn update_look() -> update::Update {
    update::look()
}

/// Pull and build, putting every line of both on the screen as it happens.
#[tauri::command]
fn update_run(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    let say = |line: &str| {
        let _ = app.emit("updating", line);
    };
    update::run(&say)
}

/// Send a failed build to the bot, with the home directory taken out of it.
///
/// Never on its own: the window asks first unless somebody has said in settings
/// that it need not, and even then this is the only thing that leaves — the
/// version, the system and the log.
#[tauri::command]
fn send_report(log: String) -> Result<String, String> {
    let said = settings::Settings::load();
    if said.server.is_empty() {
        return Err("адрес бота не задан — отправлять некуда".to_owned());
    }
    let hardware = machine::Hardware::read();
    let what = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": hardware.os,
        "cpu": hardware.cpu,
        "log": update::tidy(&log),
    });
    bot::Bot::new(&said.server, &said.token, &said.name)
        .and_then(|bot| bot.report(&what))
        .map(|()| "отправлено разработчику".to_owned())
        .map_err(|refused| refused.to_string())
}

/// What this application is made of, what it needs from outside, and what is
/// only planned.
#[tauri::command]
fn modules() -> Vec<library::Module> {
    library::modules(&settings::Settings::load())
}

/// Whether the bot agrees this machine could work. One network call, asked
/// only when somebody is looking at the readiness list.
#[tauri::command]
fn handshake() -> check::Row {
    check::handshake(&settings::Settings::load())
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
            about,
            judged,
            pick_folder,
            pick_replay,
            pick_skin,
            install_skin,
            skin_pictures,
            build_reel,
            modules,
            update_look,
            update_run,
            send_report,
            handshake
        ])
        .run(tauri::generate_context!())
        .expect("the window could not be opened");
}
