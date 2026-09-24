use dossier_native::{gallery, settings, App, WINDOW};
use iced::{window, Size};

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|a| a == "--gallery") {
        let dir = args.get(at + 1).cloned().unwrap_or_else(|| "gallery".to_owned());
        match gallery::write(std::path::Path::new(&dir)).and_then(|n| gallery::write_main(std::path::Path::new(&dir)).map(|m| n + m)) {
            Ok(n) => println!("{n} frames in {dir}"),
            Err(why) => {
                eprintln!("{why}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if let Some(at) = args.iter().position(|a| a == "--icons") {
        let dir = args.get(at + 1).cloned().unwrap_or_else(|| "assets/icon".to_owned());
        match dossier_native::icon::write(std::path::Path::new(&dir)) {
            Ok(n) => println!("{n} icons in {dir}"),
            Err(why) => {
                eprintln!("{why}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if let Some(rehearsal) = dossier_native::Rehearsal::from_args(&args) {
        let _ = dossier_native::REHEARSAL.set(rehearsal);
    }
    if let Some(at) = args.iter().position(|a| a == "--shot") {
        let root = std::path::PathBuf::from(args.get(at + 1).cloned().unwrap_or_else(|| ".".to_owned()));
        let out = std::path::PathBuf::from(args.get(at + 2).cloned().unwrap_or_else(|| "main.png".to_owned()));
        let lang = if args.iter().any(|a| a == "--ru") { dossier_native::lang::Lang::Ru } else { dossier_native::lang::Lang::En };
        if let Err(why) = gallery::shot_folder(&root, lang, &out) {
            eprintln!("{why}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(at) = args.iter().position(|a| a == "--render") {
        let replay = std::path::PathBuf::from(args.get(at + 1).cloned().unwrap_or_default());
        let folder = replay.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let Some(source) = dossier_native::sources::folder_at(&folder) else {
            eprintln!("no replays beside {}", replay.display());
            std::process::exit(1);
        };
        let library = dossier_native::library::read(&[source]);
        let Some(entry) = library.entries.iter().find(|e| e.path == replay) else {
            eprintln!("{} is not in the library", replay.display());
            std::process::exit(1);
        };
        let (Some(map), Some(ffmpeg)) = (&entry.map, dossier_native::checks::ffmpeg_on_path()) else {
            eprintln!("no map on disk or no ffmpeg");
            std::process::exit(1);
        };
        let out = std::env::temp_dir().join(dossier_native::render::file_name(&entry.player, &map.line()));
        let said = dossier_native::settings::Settings::load();
        let height: u32 = std::env::var("DOSSIER_RENDER_HEIGHT").ok().and_then(|h| h.parse().ok()).unwrap_or(1080);
        let ask = dossier_native::render::Ask {
            replay: entry.path.clone(),
            map: map.file.clone(),
            map_hash: entry.map_hash.clone(),
            ffmpeg,
            out,
            size: ((height * 16 / 9 + 1) & !1, height),
            fps: std::env::var("DOSSIER_RENDER_FPS").ok().and_then(|f| f.parse().ok()).unwrap_or(60),
            crf: 20,
            skin: said.skin.clone(),
            music_level: said.music_level,
            hitsound_level: said.hitsound_level,
            play: dossier_native::render::Play {
                hud: said.hud,
                cursor_grows: said.cursor_grows,
                dim: (said.background_dim * 100.0).round() as u32,
                blur: (said.background_blur * 100.0).round() as u32,
                map_sounds: said.map_sounds,
                skin_sounds: said.skin_sounds,
            },
        };
        println!("skin {:?} · {:?}", ask.skin, ask.play);
        let started = std::time::Instant::now();
        dossier_native::render::perform(ask, &mut |step| {
            println!("{:>7.2?} {step:?}", started.elapsed());
            true
        });
        return Ok(());
    }
    if let Some(at) = args.iter().position(|a| a == "--frame") {
        let replay = std::path::PathBuf::from(args.get(at + 1).cloned().unwrap_or_default());
        let when: f64 = args.get(at + 2).and_then(|t| t.parse().ok()).unwrap_or(10_000.0);
        let out = std::path::PathBuf::from(args.get(at + 3).cloned().unwrap_or_else(|| "frame.png".to_owned()));
        let folder = replay.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let Some(source) = dossier_native::sources::folder_at(&folder) else {
            eprintln!("no replays beside {}", replay.display());
            std::process::exit(1);
        };
        let library = dossier_native::library::read(&[source]);
        let Some(entry) = library.entries.iter().find(|e| e.path == replay) else {
            eprintln!("{} is not in the library", replay.display());
            std::process::exit(1);
        };
        let Some(map) = &entry.map else {
            eprintln!("no map on disk");
            std::process::exit(1);
        };
        let said = dossier_native::settings::Settings::load();
        if args.get(at + 2).map(String::as_str) == Some("list") {
            for (when, what) in dossier_native::render::verdicts(&entry.path, &map.file, &entry.map_hash).unwrap_or_default() {
                println!("{when:>9.0} ms  {what}");
            }
            return Ok(());
        }
        match dossier_native::render::still(&entry.path, &map.file, &entry.map_hash, said.skin.as_deref(), said.hud, when, (1280, 720)) {
            Ok(png) => {
                let _ = std::fs::write(&out, png);
                println!("{} at {when} ms", out.display());
            }
            Err(why) => eprintln!("{why}"),
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--skins") {
        let settings = dossier_native::settings::Settings::load();
        let started = std::time::Instant::now();
        let found = dossier_native::settings::hunt_skins(&settings.sources, &settings.own_skins);
        println!("{} skins, {:.1?}", found.len(), started.elapsed());
        for path in &found {
            let what = if dossier_native::settings::is_skin_file(path) { "osk" } else { "folder" };
            let sounds = dossier_audio::SamplePack::load(path);
            println!(
                "  {what} · {} · {} sounds of its own, {} not taken · {}",
                dossier_native::settings::skin_name(path),
                sounds.len(),
                sounds.unused().len(),
                path.display()
            );
            if args.iter().any(|a| a == "--loud") {
                println!("    not taken: {}", sounds.unused().join(", "));
            }
        }
        return Ok(());
    }

    if let Some(at) = args.iter().position(|a| a == "--library") {
        let root = std::path::PathBuf::from(args.get(at + 1).cloned().unwrap_or_else(|| ".".to_owned()));
        let Some(source) = dossier_native::sources::folder_at(&root) else {
            eprintln!("no replays in {}", root.display());
            std::process::exit(1);
        };
        let started = std::time::Instant::now();
        let library = dossier_native::library::read(&[source]);
        let with_map = library.entries.iter().filter(|e| e.map.is_some()).count();
        println!("{} replays, {} with their map, {} maps indexed, {:.1?}", library.entries.len(), with_map, library.maps, started.elapsed());
        for entry in library.entries.iter().take(8) {
            println!(
                "  {} · {} · {} · {:.2}% · {} · {} · {}",
                entry.played_at,
                entry.player,
                entry.map_line().unwrap_or_else(|| format!("? {}", &entry.map_hash[..8])),
                entry.accuracy,
                entry.grade.letter(),
                entry.outcome.mark(),
                entry.mods.join("")
            );
        }
        return Ok(());
    }
    if dossier_native::REHEARSAL.get().is_none() && !settings::first_run() && dossier_native::updates::on_launch(settings::Settings::load().quiet_updates) {
        return Ok(());
    }
    iced::application(App::boot, App::update, App::view)
        .title("Dossier")
        .settings(settings())
        .window(window::Settings {
            size: std::env::var("DOSSIER_WINDOW")
                .ok()
                .and_then(|said| {
                    let (w, h) = said.split_once('x')?;
                    Some(Size::new(w.trim().parse().ok()?, h.trim().parse().ok()?))
                })
                .unwrap_or(WINDOW),
            min_size: Some(Size::new(760.0, 560.0)),
            icon: dossier_native::icon::window(),
            position: window::Position::Centered,
            ..window::Settings::default()
        })
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}
