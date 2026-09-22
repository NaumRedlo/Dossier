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
        let ask = dossier_native::render::Ask { replay: entry.path.clone(), map: map.file.clone(), map_hash: entry.map_hash.clone(), ffmpeg, out, size: dossier_native::render::SIZE, fps: dossier_native::render::FPS as u32, crf: 20, skin: None, music_level: 1.0, hitsound_level: 1.0 };
        let started = std::time::Instant::now();
        dossier_native::render::perform(ask, &mut |step| {
            println!("{:>7.2?} {step:?}", started.elapsed());
            true
        });
        return Ok(());
    }
    if args.iter().any(|a| a == "--skins") {
        let settings = dossier_native::settings::Settings::load();
        let started = std::time::Instant::now();
        let found = dossier_native::settings::hunt_skins(&settings.sources, &settings.own_skins);
        println!("{} skins, {:.1?}", found.len(), started.elapsed());
        for path in &found {
            let what = if dossier_native::settings::is_skin_file(path) { "osk" } else { "folder" };
            println!("  {what} · {} · {}", dossier_native::settings::skin_name(path), path.display());
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
    iced::application(App::boot, App::update, App::view)
        .title("Dossier")
        .settings(settings())
        .window(window::Settings {
            size: WINDOW,
            min_size: Some(Size::new(760.0, 560.0)),
            position: window::Position::Centered,
            ..window::Settings::default()
        })
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}
