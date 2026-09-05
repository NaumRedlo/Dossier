#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bench;
mod bot;
mod check;
mod draw;
mod library;
mod link;
mod logbook;
mod look;
mod machine;
mod pick;
mod pics;
mod play;
mod reel;
mod settings;
mod update;
mod work;

#[tauri::command(async)]
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
    done.map(|path| finished("рендер", &path, &told))
}

fn finished(what: &str, path: &std::path::Path, told: &draw::Told) -> Drawn {
    let said = told.said();
    logbook::note(&format!("{what}: {}", path.display()), &said);
    Drawn {
        path: path.display().to_string(),
        said,
    }
}

#[derive(serde::Serialize)]
struct Drawn {
    path: String,
    said: Vec<String>,
}

#[tauri::command(async)]
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

#[tauri::command(async)]
fn profile() -> machine::Profile {
    machine::profile()
}

#[tauri::command(async)]
fn settings_read() -> settings::Settings {
    settings::Settings::load()
}

#[tauri::command(async)]
fn first_run() -> bool {
    settings::Settings::load().first_run()
}

#[tauri::command(async)]
fn settings_write(said: settings::Settings) -> Result<(), String> {
    said.save()
}

#[tauri::command(async)]
fn skins() -> Vec<String> {
    library::skins(&settings::Settings::load())
}

#[tauri::command(async)]
fn shelves() -> library::Library {
    library::look(&settings::Settings::load())
}

#[tauri::command(async)]
fn my_replays(most: Option<usize>) -> Vec<library::Played> {
    library::played(&settings::Settings::load(), most.unwrap_or(60))
}

#[tauri::command(async)]
fn farm() -> Result<bot::Farm, String> {
    let said = settings::Settings::load();
    bot::Bot::new(&said.server, &said.token, &said.name)
        .and_then(|bot| bot.farm())
        .map_err(|refused| refused.to_string())
}

#[tauri::command(async)]
fn ready() -> Vec<check::Row> {
    check::ready()
}

#[tauri::command(async)]
fn preview(replay: String) -> Result<play::Scene, String> {
    let said = settings::Settings::load();
    play::preview(
        std::path::Path::new(&replay),
        Some(std::path::Path::new(&said.songs)),
        6.0,
    )
}

#[tauri::command(async)]
fn judged(replay: String) -> Result<play::Scene, String> {
    let said = settings::Settings::load();
    let songs = std::path::PathBuf::from(&said.songs);
    let songs = songs.is_dir().then_some(songs.as_path());
    play::open(std::path::Path::new(&replay), songs)
}

#[tauri::command(async)]
fn pick_folder(prompt: String) -> Result<Option<String>, String> {
    pick::folder(&prompt)
}

#[tauri::command(async)]
fn install_skin(path: String) -> Result<String, String> {
    library::install_skin(&settings::Settings::load(), std::path::Path::new(&path))
}

#[tauri::command(async)]
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

#[tauri::command(async)]
fn pick_skin(prompt: String) -> Result<Option<String>, String> {
    pick::file(&prompt, "osk")
}

#[tauri::command(async)]
fn pick_replay(prompt: String) -> Result<Option<String>, String> {
    pick::file(&prompt, "osr")
}

#[tauri::command(async)]
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
    done.map(|path| finished("студия", &path, &told))
}

#[tauri::command(async)]
fn update_look() -> update::Update {
    update::look()
}

#[tauri::command(async)]
fn update_run(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    let say = |line: &str| {
        let _ = app.emit("updating", line);
    };
    update::run(&say)
}

#[tauri::command(async)]
fn send_report(log: String, kind: Option<String>) -> Result<String, String> {
    let said = settings::Settings::load();
    if said.server.is_empty() {
        return Err("адрес бота не задан — отправлять некуда".to_owned());
    }
    let hardware = machine::Hardware::read();
    let what = serde_json::json!({
        "kind": kind.unwrap_or_else(|| "build".to_owned()),
        "version": env!("CARGO_PKG_VERSION"),
        "os": hardware.os,
        "cpu": hardware.cpu,
        "log": update::tidy(&log),
    });
    bot::Bot::new(&said.server, &said.token, &said.name)
        .and_then(|bot| bot.report(&what))
        .map(|()| "отправлено разработчику".to_owned())
        .map_err(|refused| {
            let said = refused.to_string();

            if said.contains("404") {
                "у бота ещё нет приёмника отчётов (POST /render/report) —                  отправлять некуда, пока он не появится"
                    .to_owned()
            } else {
                said
            }
        })
}

#[tauri::command(async)]
fn modules() -> Vec<library::Module> {
    library::modules(&settings::Settings::load())
}

#[tauri::command(async)]
fn handshake() -> check::Row {
    check::handshake(&settings::Settings::load())
}

#[tauri::command(async)]
fn open_link(url: String) -> Result<(), String> {
    link::open(&url)
}

#[tauri::command(async)]
fn reveal_file(path: String) -> Result<(), String> {
    link::reveal(std::path::Path::new(&path))
}

#[tauri::command(async)]
fn play_file(path: String) -> Result<(), String> {
    link::play(std::path::Path::new(&path))
}

#[tauri::command(async)]
fn log_open() -> Result<(), String> {
    logbook::open()
}

#[tauri::command(async)]
fn log_where() -> String {
    logbook::path().display().to_string()
}

#[tauri::command(async)]
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
            preview,
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
            reveal_file,
            play_file,
            log_open,
            log_where,
            handshake
        ])
        .run(tauri::generate_context!())
        .expect("the window could not be opened");
}
