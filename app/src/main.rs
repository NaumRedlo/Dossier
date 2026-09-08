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
mod mirror;
mod pick;
mod play;
mod reel;
mod settings;
mod show;
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

async fn off_thread<T, F>(work: F) -> Result<T, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|why| format!("работа не довелась: {why}"))
}

#[derive(serde::Serialize)]
struct Shown {
    show: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    still: Option<String>,

    #[serde(flatten)]
    scene: play::Scene,
}

fn wanted(replay: String, skin: Option<String>, fine: Option<draw::Fine>) -> show::Wanted {
    let said = settings::Settings::load();
    let songs = std::path::PathBuf::from(&said.songs);
    let folder = skin.filter(|n| !n.is_empty()).unwrap_or(said.skin.clone());
    let chosen = (!folder.is_empty() && !said.skins.is_empty())
        .then(|| std::path::Path::new(&said.skins).join(folder))
        .filter(|path| path.is_dir());
    show::Wanted {
        replay: std::path::PathBuf::from(replay),
        songs: songs.is_dir().then_some(songs),
        skin: chosen,
        fine: fine.unwrap_or_default(),
    }
}

fn shown(
    replay: String,
    skin: Option<String>,
    fine: Option<draw::Fine>,
    seconds: Option<f64>,
) -> Result<Shown, String> {
    let id = show::open(wanted(replay, skin, fine))?;
    match show::facts(id, seconds) {
        Ok(scene) => Ok(Shown {
            show: id,
            still: None,
            scene,
        }),
        Err(why) => {
            show::shut(id);
            Err(why)
        }
    }
}

#[tauri::command(async)]
async fn preview(
    replay: String,
    skin: Option<String>,
    fine: Option<draw::Fine>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Shown, String> {
    off_thread(move || preview_now(replay, skin, fine, width, height)).await?
}

fn preview_now(
    replay: String,
    skin: Option<String>,
    fine: Option<draw::Fine>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Shown, String> {
    let id = show::open(wanted(replay, skin, fine))?;
    let facts = show::facts(id, Some(6.0));
    let still = facts.as_ref().ok().and_then(|scene| {
        let piece = scene.parts.first()?;
        let png = show::frame(
            id,
            (piece[0] + piece[1]) / 2.0,
            width.unwrap_or(480).clamp(16, 1920),
            height.unwrap_or(360).clamp(16, 1080),
        )?;
        Some(show::as_data_url(&png))
    });
    show::shut(id);
    facts.map(|scene| Shown {
        show: 0,
        still,
        scene,
    })
}

#[tauri::command(async)]
async fn judged(
    replay: String,
    skin: Option<String>,
    fine: Option<draw::Fine>,
) -> Result<Shown, String> {
    off_thread(move || shown(replay, skin, fine, None)).await?
}

#[derive(serde::Serialize)]
struct MapHere {
    hash: String,
    here: bool,
}

#[tauri::command(async)]
async fn map_here(replay: String) -> Result<MapHere, String> {
    off_thread(move || map_here_now(replay)).await?
}

fn map_here_now(replay: String) -> Result<MapHere, String> {
    let bytes = std::fs::read(&replay).map_err(|why| format!("реплей не читается: {why}"))?;
    let played = dossier_replay::Replay::heading(&bytes).map_err(|why| format!("{why}"))?;
    let said = settings::Settings::load();
    let songs = std::path::PathBuf::from(&said.songs);
    let here = songs.is_dir()
        && dossier_produce::locate::search_dir(&songs, &played.beatmap_hash)
            .ok()
            .flatten()
            .is_some();
    Ok(MapHere {
        hash: played.beatmap_hash,
        here,
    })
}

#[tauri::command(async)]
async fn fetch_map(app: tauri::AppHandle, replay: String) -> Result<mirror::Found, String> {
    off_thread(move || fetch_map_now(&app, replay)).await?
}

fn fetch_map_now(app: &tauri::AppHandle, replay: String) -> Result<mirror::Found, String> {
    use tauri::Emitter;

    let bytes = std::fs::read(&replay).map_err(|why| format!("реплей не читается: {why}"))?;
    let played = dossier_replay::Replay::heading(&bytes).map_err(|why| format!("{why}"))?;
    let said = settings::Settings::load();

    let sending = app.clone();
    let say = move |step: &str, got: u64, of: u64| {
        let _ = sending.emit(
            "fetching",
            serde_json::json!({ "replay": replay, "step": step, "got": got, "of": of }),
        );
    };
    let found = mirror::bring(
        &played.beatmap_hash,
        std::path::Path::new(&said.songs),
        &say,
    );
    match &found {
        Ok(one) => logbook::note(
            "карта скачана",
            &[format!("{} — {} [{}]", one.artist, one.title, one.version)],
        ),
        Err(why) => logbook::note("карта не скачалась", std::slice::from_ref(why)),
    }
    found
}

#[tauri::command(async)]
fn drop_replay(path: String) -> Result<(), String> {
    library::drop_replay(&settings::Settings::load(), std::path::Path::new(&path))?;
    logbook::note("реплей удалён", std::slice::from_ref(&path));
    Ok(())
}

#[tauri::command(async)]
async fn show_open(
    replay: String,
    skin: Option<String>,
    fine: Option<draw::Fine>,
) -> Result<u64, String> {
    off_thread(move || show::open(wanted(replay, skin, fine))).await?
}

#[tauri::command(async)]
fn show_shut(show: u64) {
    show::shut(show);
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
fn clone_skin(name: String) -> Result<String, String> {
    library::clone_skin(&settings::Settings::load(), &name)
}

#[tauri::command(async)]
fn export_skin(name: String, into: String) -> Result<String, String> {
    library::export_skin(
        &settings::Settings::load(),
        &name,
        std::path::Path::new(&into),
    )
}

#[tauri::command(async)]
fn remove_skin(name: String) -> Result<(), String> {
    library::remove_skin(&settings::Settings::load(), &name)
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

fn open_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItem::with_id(app, "open", "Открыть Dossier", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выйти", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => open_window(app),
            "quit" => app.exit(0),
            _ => {}
        });
    match tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")) {
        Ok(icon) => tray = tray.icon(icon),
        Err(_) => {
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
        }
    }
    tray.build(app)?;
    Ok(())
}

fn shape_window(app: &tauri::App) {
    use tauri::Manager;
    let said = settings::Settings::load();
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let wanted = said.window_wanted();
    if wanted == "full" {
        let _ = window.set_fullscreen(true);
        return;
    }
    if let Some((wide, high)) = wanted.split_once('x') {
        if let (Ok(wide), Ok(high)) = (wide.trim().parse::<f64>(), high.trim().parse::<f64>()) {
            let _ = window.set_size(tauri::LogicalSize::new(wide, high));
            let _ = window.center();
        }
    }
}

fn serve_frames(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    let uri = request.uri().to_string();
    std::thread::spawn(move || {
        let drawn = show::asked_for(&uri).and_then(|(id, ms, w, h)| show::frame(id, ms, w, h));
        let answer = match drawn {
            Some(png) => tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", "image/png")
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store")
                .body(png),
            None => tauri::http::Response::builder()
                .status(404)
                .header("Content-Type", "text/plain")
                .body(Vec::new()),
        };
        if let Ok(answer) = answer {
            responder.respond(answer);
        }
    });
}

fn main() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("frame", |_app, request, responder| {
            serve_frames(request, responder);
        })
        .setup(|app| {
            shape_window(app);
            build_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if settings::Settings::load().hides_on_close() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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
            clone_skin,
            export_skin,
            remove_skin,
            show_open,
            show_shut,
            map_here,
            fetch_map,
            drop_replay,
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
