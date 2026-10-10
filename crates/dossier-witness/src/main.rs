#[cfg(windows)]
fn main() {
    use dossier_witness::memory::Memory;
    use dossier_witness::{stable, windows};

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--serve" || arg == "--overlay" || arg == "--hud") {
        let player = args.iter().position(|arg| arg == "--player").and_then(|at| args.get(at + 1)).cloned().unwrap_or_default();
        let leash = args.iter().any(|arg| arg == "--leash").then(|| std::env::current_exe().ok().and_then(|program| dossier_witness::leash::beside(&program))).flatten();
        let overlay_mode = args.iter().any(|arg| arg == "--overlay");
        let english = args.iter().position(|arg| arg == "--lang").and_then(|at| args.get(at + 1)).is_some_and(|said| said == "en");
        let hud = (!overlay_mode && args.iter().any(|arg| arg == "--hud")).then(|| (if english { dossier_hud::Lang::En } else { dossier_hud::Lang::Ru }, args.iter().any(|arg| arg == "--offer")));
        serve(&player, leash.as_deref(), overlay_mode, hud);
        return;
    }
    if let Some(at) = args.iter().position(|arg| arg == "--library") {
        let md5 = args.iter().position(|arg| arg == "--md5").and_then(|at| args.get(at + 1));
        library_report(args.get(at + 1), md5);
        return;
    }
    let rounds: usize = args.iter().position(|arg| arg == "--watch").and_then(|at| args.get(at + 1)).and_then(|said| said.parse().ok()).unwrap_or(1);
    let name = args.iter().position(|arg| arg == "--process").and_then(|at| args.get(at + 1)).cloned().unwrap_or_else(|| "osu!.exe".to_owned());
    let found = windows::processes_named(&name);
    println!("witness: {} named {name}: {found:?}", found.len());
    let Some(process) = found.iter().find_map(|pid| windows::Process::open(*pid)) else {
        println!("witness: no client to read");
        std::process::exit(2);
    };
    let regions = process.regions();
    let code = regions.iter().filter(|region| region.executable).count();
    println!("witness: pid {} has {} readable regions, {} of them code, {} MB in all", process.pid, regions.len(), code, regions.iter().map(|region| region.size).sum::<u64>() >> 20);
    if args.iter().any(|arg| arg == "--regions") {
        let mut kinds: std::collections::BTreeMap<(u32, u32), (usize, u64)> = std::collections::BTreeMap::new();
        for region in &regions {
            let seen = kinds.entry((region.protect, region.kind)).or_default();
            seen.0 += 1;
            seen.1 += region.size;
        }
        for ((protect, kind), (count, size)) in kinds {
            println!("witness: protect {protect:#x} kind {kind:#x}: {count} regions, {} KB", size >> 10);
        }
    }
    if args.iter().any(|arg| arg == "--windows") {
        for window in windows::windows_of(process.pid) {
            println!("witness: window {window:?}");
        }
    }
    if args.iter().any(|arg| arg == "--signatures") {
        for (name, said) in [("base", stable::BASE), ("status", stable::STATUS), ("play time", stable::PLAY_TIME), ("rulesets", stable::RULESETS), ("replay", stable::REPLAY)] {
            let pattern = dossier_witness::scan::Pattern::parse(said).expect("a pattern");
            let code = dossier_witness::scan::find_all(&process, &pattern, true, 8);
            let anywhere = dossier_witness::scan::find_all(&process, &pattern, false, 8);
            println!("witness: {name}: in code {code:x?}, anywhere {anywhere:x?}");
        }
    }
    if let Some(at) = args.iter().position(|arg| arg == "--report") {
        let seconds: u64 = args.get(at + 1).and_then(|said| said.parse().ok()).unwrap_or(60);
        report(&process, seconds);
        return;
    }
    let anchors = match stable::anchors(&process) {
        Ok(anchors) => anchors,
        Err(lost) => {
            println!("witness: the client is not the one these signatures know: {lost:?} was not found");
            std::process::exit(3);
        }
    };
    println!("witness: anchors {anchors:x?}");
    if let Some(out) = args.iter().position(|arg| arg == "--record").and_then(|at| args.get(at + 1)) {
        let player = args.iter().position(|arg| arg == "--player").and_then(|at| args.get(at + 1)).cloned().unwrap_or_default();
        record(&process, &anchors, std::path::Path::new(out), &player);
        return;
    }
    let exploring = args.iter().any(|arg| arg == "--explore");
    let mut last = String::new();
    for round in 0..rounds {
        match stable::glance(&process, &anchors) {
            Some(seen) => {
                let said = format!("{:?} {:?} {:?}", seen.mode, seen.map.as_ref().map(|map| (&map.md5, &map.title, &map.version)), seen.play);
                if !exploring || said != last || round % 4 == 0 {
                    println!("witness: [{round}] t={} {said}", seen.time_ms);
                }
                last = said;
            }
            None => println!("witness: nothing could be read"),
        }
        if exploring && round % 4 == 0 {
            explore(&process, &anchors);
        }
        if round + 1 < rounds {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

#[cfg(windows)]
fn report(process: &dossier_witness::windows::Process, seconds: u64) {
    use dossier_witness::{report::Watch, scan, stable};

    let client = client_of(process);
    println!("witness report: this program is {}", std::env::current_exe().map_or_else(|_| "unknown".to_owned(), |path| path.display().to_string()));
    println!("witness report: the client runs from {}", process.folder().map_or_else(|| "a folder the system did not name".to_owned(), |folder| folder.display().to_string()));
    println!("witness report: the client's build is {}, its player {}", if client.build.is_empty() { "not told" } else { client.build.as_str() }, if client.player.is_empty() { "not told" } else { client.player.as_str() });
    for (name, said) in [("base", stable::BASE), ("screen", stable::STATUS), ("play time", stable::PLAY_TIME), ("rulesets", stable::RULESETS), ("replay flag", stable::REPLAY)] {
        let pattern = scan::Pattern::parse(said).expect("a pattern");
        println!("witness report: signature {name}: {} found in code", scan::find_all(process, &pattern, true, 8).len());
    }
    let anchors = match stable::anchors(process) {
        Ok(anchors) => anchors,
        Err(lost) => {
            println!("witness report: the client is not the one these signatures know: {lost:?} was not found");
            println!("witness report: a client that has not got past its first window has not compiled the code the signatures sit in yet; try again once the menu is shown");
            return;
        }
    };
    println!("witness report: every anchor needed was found, the replay flag one {}", if anchors.replay.is_some() { "too" } else { "was not" });
    println!("witness report: for {seconds} s, open song select, play a map for a while and let it fail, then watch any replay");
    let mut watch = Watch::default();
    for second in 0..seconds {
        if let Some(line) = watch.observe(process, &anchors) {
            println!("witness report: [{second:>3} s] {line}");
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    for line in watch.summary() {
        println!("witness report: {line}");
    }
}

#[cfg(windows)]
fn library_report(path: Option<&String>, md5: Option<&String>) {
    use dossier_witness::beatmaps::Library;

    let Some(path) = path else {
        println!("witness: --library wants the path of an osu!.db");
        return;
    };
    let Some(library) = Library::open(std::path::Path::new(path)) else {
        println!("witness: {path} is not a library this program can read");
        return;
    };
    println!("witness: library version {}, player {}, {} of {} maps read, {}", library.version, library.player, library.len(), library.declared, if library.complete { "whole" } else { "cut short" });
    let standard = library.maps().filter(|map| map.mode == 0).count();
    let with_stars = library.maps().filter(|map| map.mode == 0 && !map.stars.is_empty()).count();
    println!("witness: {standard} maps of osu!standard, {with_stars} of them with stars for the mods");
    if let Some(md5) = md5 {
        match library.get(md5) {
            Some(map) => println!("witness: {md5} is {} [{}], {:?}", map.id, map.version, map.facts(0)),
            None => println!("witness: {md5} is not in the library"),
        }
    }
}

#[cfg(windows)]
fn say(line: String) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
        std::process::exit(0);
    }
}

#[cfg(windows)]
fn let_go(leash: Option<&std::path::Path>) {
    if leash.is_some_and(|leash| !dossier_witness::leash::held(leash, std::time::SystemTime::now())) {
        std::process::exit(0);
    }
}

#[cfg(windows)]
struct Shown {
    hud: dossier_witness::hud::Hud,
    panes: dossier_witness::panes::Panes,
    pid: u32,
    window: Option<isize>,
    fresh: bool,
    place: Option<(i32, i32, i32, i32)>,
    said: String,
}

#[cfg(windows)]
impl Shown {
    fn new(pid: u32, lang: dossier_hud::Lang, offers: bool) -> Shown {
        Shown { hud: dossier_witness::hud::Hud::new(lang, offers), panes: dossier_witness::panes::Panes::new(), pid, window: None, fresh: true, place: None, said: String::new() }
    }

    fn told(&mut self, frame: &dossier_overlay::Frame) {
        self.hud.told(frame, self.pid);
        self.fresh = true;
    }

    fn kept(&mut self, passed: bool) {
        self.hud.kept(passed);
        self.fresh = true;
    }

    fn frame(&mut self) {
        use dossier_witness::panes;
        self.panes.pump();
        if self.window.is_none_or(|window| panes::area(window).is_none()) {
            self.window = panes::game_window(self.pid);
        }
        let keys = self.window.map_or_else(Default::default, panes::keys);
        if self.hud.step(keys, std::time::Instant::now()) {
            eprintln!("witness hud: the send key was held for its time; nothing is sent yet");
            self.fresh = true;
        }
        let place = self.window.and_then(panes::area).filter(|_| keys.front);
        let said = match place {
            Some((x, y, wide, high)) => {
                if self.fresh || self.place != place || self.hud.moving() {
                    let sprites = self.hud.sprites(dossier_overlay::Viewport { width: wide as u32, height: high as u32, scale: 1.0 });
                    self.panes.show((x, y), &sprites);
                    self.fresh = false;
                }
                format!("{} plates over the game, its picture is {wide}x{high} at {x}, {y}", self.panes.count())
            }
            None => {
                self.panes.hide();
                self.fresh = true;
                if self.window.is_some() { "hidden: the game is not in front".to_owned() } else { "hidden: the game has no window yet".to_owned() }
            }
        };
        self.place = place;
        if said != self.said {
            eprintln!("witness hud: {said}");
            self.said = said;
        }
    }
}

#[cfg(windows)]
fn rest(shown: Option<&mut Shown>, time: std::time::Duration) {
    let Some(shown) = shown else {
        std::thread::sleep(time);
        return;
    };
    let until = std::time::Instant::now() + time;
    loop {
        shown.frame();
        let now = std::time::Instant::now();
        if now >= until {
            return;
        }
        std::thread::sleep((until - now).min(std::time::Duration::from_millis(if shown.hud.moving() { 16 } else { 50 })));
    }
}

#[cfg(windows)]
fn serve(player: &str, leash: Option<&std::path::Path>, overlay_mode: bool, hud: Option<(dossier_hud::Lang, bool)>) {
    use dossier_witness::memory::Reads;
    use dossier_witness::{osr, stable, windows, wire};
    use std::time::{Duration, Instant};

    let legacy = |line| { if !overlay_mode { say(line); } };
    let emit = |frame: dossier_overlay::Frame| {
        if let Ok(line) = frame.encode() { say(line.trim_end().to_owned()); }
    };
    let mut session = 0u64;
    const NAME: &str = "osu!.exe";
    let mut idle_told = false;
    loop {
        let_go(leash);
        let Some(process) = windows::processes_named(NAME).iter().find_map(|pid| windows::Process::open(*pid)) else {
            legacy(wire::plain(if idle_told { "alive" } else { "waiting" }));
            idle_told = true;
            std::thread::sleep(Duration::from_secs(if idle_told { 3 } else { 1 }));
            continue;
        };
        idle_told = false;
        session += 1;
        let client = client_of(&process);
        let mut overlay = dossier_witness::overlay::Publisher::new(session).knowing(client.meter(), process.folder().map(|folder| client.songs_in(&folder)));
        if overlay_mode { emit(overlay.hello(process.pid, &client.build)); }
        legacy(wire::attached(process.pid, &client));
        let mut loading_told = false;
        let anchors = loop {
            if let Ok(anchors) = stable::anchors(&process) {
                break Some(anchors);
            }
            if !windows::processes_named(NAME).contains(&process.pid) {
                break None;
            }
            let_go(leash);
            legacy(wire::plain(if loading_told { "alive" } else { "loading" }));
            loading_told = true;
            std::thread::sleep(Duration::from_secs(2));
        };
        let Some(anchors) = anchors else {
            if overlay_mode { emit(overlay.disconnected()); }
            legacy(wire::plain("gone"));
            continue;
        };
        let mut shelf = process.folder().map(|folder| dossier_witness::beatmaps::Shelf::beside(&folder));
        let mut recorder = stable::Recorder::default();
        let mut shown = hud.map(|(lang, offers)| Shown::new(process.pid, lang, offers));
        let mut last_state = String::new();
        let mut told_at = Instant::now();
        let mut alive_at = Instant::now();
        let mut checked_at = Instant::now();
        loop {
            if checked_at.elapsed() >= Duration::from_secs(2) {
                let_go(leash);
                checked_at = Instant::now();
            }
            if process.u32(anchors.status).is_none() {
                if overlay_mode { emit(overlay.disconnected()); }
                legacy(wire::plain("gone"));
                break;
            }
            let seen = stable::glance(&process, &anchors);
            if overlay_mode {
                if let Some(seen) = &seen { emit(overlay.snapshot(seen)); }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            if let (Some(shown), Some(seen)) = (shown.as_mut(), &seen) {
                shown.told(&overlay.snapshot(seen));
            }
            if let Some(seen) = &seen {
                let facts = seen.map.as_ref().and_then(|map| shelf.as_mut()?.map(&map.md5).map(|known| known.facts(0)));
                let said = wire::state(seen, facts.as_ref());
                if said != last_state {
                    legacy(said.clone());
                    last_state = said;
                }
            }
            if let Some(take) = recorder.poll(&process, &anchors) {
                if take.frames.len() >= FRAMES_LEAST {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
                    let facts = shelf.as_mut().and_then(|shelf| shelf.map(&take.map.md5).map(|known| known.facts(take.play.mods)));
                    legacy(wire::kept(&take, &osr::file_name(&take, &client, player, now), &osr::write(&take, &client, player, now), facts.as_ref()));
                    if let Some(shown) = shown.as_mut() {
                        shown.kept(take.passed && take.watched != Some(true));
                    }
                }
            }
            if let (Some(take), Some(seen)) = (recorder.recording(), &seen) {
                if told_at.elapsed() >= Duration::from_secs(1) {
                    legacy(wire::playing(take, seen.time_ms));
                    told_at = Instant::now();
                    alive_at = Instant::now();
                }
            }
            if alive_at.elapsed() >= Duration::from_secs(5) {
                legacy(wire::plain("alive"));
                alive_at = Instant::now();
            }
            rest(shown.as_mut(), Duration::from_millis(100));
        }
    }
}

#[cfg(windows)]
fn client_of(process: &dossier_witness::windows::Process) -> dossier_witness::client::Client {
    let user = std::env::var("USERNAME").unwrap_or_default();
    process.folder().map_or_else(dossier_witness::client::Client::default, |folder| dossier_witness::client::Client::beside(&folder, &user))
}

#[cfg(windows)]
const FRAMES_LEAST: usize = 120;

#[cfg(windows)]
fn record(process: &dossier_witness::windows::Process, anchors: &dossier_witness::stable::Anchors, out: &std::path::Path, player: &str) {
    use dossier_witness::{osr, stable};
    let _ = std::fs::create_dir_all(out);
    let client = client_of(process);
    println!("witness: the client is {} and its player {}", if client.build.is_empty() { "of an unknown build" } else { client.build.as_str() }, if client.player.is_empty() { "unnamed" } else { client.player.as_str() });
    let mut recorder = stable::Recorder::default();
    let mut told = 0usize;
    println!("witness: recording into {}", out.display());
    loop {
        if dossier_witness::memory::Reads::u32(process, anchors.status).is_none() {
            println!("witness: the client is gone");
            return;
        }
        if let Some(take) = recorder.poll(process, anchors) {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
            if take.frames.len() < FRAMES_LEAST {
                println!("witness: a play of {} frames is too short to keep", take.frames.len());
            } else {
                let file = out.join(osr::file_name(&take, &client, player, now));
                match std::fs::write(&file, osr::write(&take, &client, player, now)) {
                    Ok(()) => println!("witness: kept {} ({} frames, {} points, passed {}, failed {})", file.display(), take.frames.len(), take.play.score, take.passed, take.failed),
                    Err(why) => println!("witness: {} was not written: {why}", file.display()),
                }
            }
            told = 0;
        }
        if let Some(take) = recorder.recording() {
            if take.frames.len() / 600 > told {
                told = take.frames.len() / 600;
                println!("witness: {} frames of {} [{}], {} points", take.frames.len(), take.map.title, take.map.version, take.play.score);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn explore(process: &dossier_witness::windows::Process, anchors: &dossier_witness::stable::Anchors) {
    use dossier_witness::memory::Reads;
    let Some(score) = dossier_witness::stable::score_at(process, anchors) else {
        return;
    };
    let hex = |bytes: &[u8]| bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" ");
    println!("witness:   score at {score:x}");
    for offset in (4u64..0x70).step_by(4) {
        let Some(held) = process.pointer(score + offset) else {
            continue;
        };
        let Some((items, size)) = process.list(held) else {
            continue;
        };
        if size == 0 {
            println!("witness:   +{offset:#x}: an empty list");
            continue;
        }
        let item = |at: usize| -> String {
            let Some(word) = process.u32(items + 4 * at as u64) else {
                return "unreadable".to_owned();
            };
            match process.bytes(u64::from(word), 28).filter(|_| word > 0x1_0000) {
                Some(object) => format!("object {word:x}: {}", hex(&object)),
                None => format!("value {} ({word:#x})", word as i32),
            }
        };
        println!("witness:   +{offset:#x}: a list of {size}; first {}; last {}", item(0), item(size - 1));
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("witness runs where the client runs: on Windows, or inside the client's own Wine prefix");
    std::process::exit(2);
}
