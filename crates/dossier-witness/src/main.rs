#[cfg(windows)]
fn main() {
    use dossier_witness::memory::Memory;
    use dossier_witness::{stable, windows};

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--serve") {
        let player = args.iter().position(|arg| arg == "--player").and_then(|at| args.get(at + 1)).cloned().unwrap_or_default();
        serve(&player);
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
        for (name, said) in [("base", stable::BASE), ("status", stable::STATUS), ("play time", stable::PLAY_TIME), ("rulesets", stable::RULESETS)] {
            let pattern = dossier_witness::scan::Pattern::parse(said).expect("a pattern");
            let code = dossier_witness::scan::find_all(&process, &pattern, true, 8);
            let anywhere = dossier_witness::scan::find_all(&process, &pattern, false, 8);
            println!("witness: {name}: in code {code:x?}, anywhere {anywhere:x?}");
        }
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
fn say(line: String) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
        std::process::exit(0);
    }
}

#[cfg(windows)]
fn serve(player: &str) {
    use dossier_witness::memory::Reads;
    use dossier_witness::{osr, stable, windows, wire};
    use std::time::{Duration, Instant};

    const NAME: &str = "osu!.exe";
    let mut idle_told = false;
    loop {
        let Some(process) = windows::processes_named(NAME).iter().find_map(|pid| windows::Process::open(*pid)) else {
            say(wire::plain(if idle_told { "alive" } else { "waiting" }));
            idle_told = true;
            std::thread::sleep(Duration::from_secs(if idle_told { 3 } else { 1 }));
            continue;
        };
        idle_told = false;
        say(wire::attached(process.pid));
        let mut loading_told = false;
        let anchors = loop {
            if let Ok(anchors) = stable::anchors(&process) {
                break Some(anchors);
            }
            if !windows::processes_named(NAME).contains(&process.pid) {
                break None;
            }
            say(wire::plain(if loading_told { "alive" } else { "loading" }));
            loading_told = true;
            std::thread::sleep(Duration::from_secs(2));
        };
        let Some(anchors) = anchors else {
            say(wire::plain("gone"));
            continue;
        };
        let mut recorder = stable::Recorder::default();
        let mut last_state = String::new();
        let mut told_at = Instant::now();
        let mut alive_at = Instant::now();
        loop {
            if process.u32(anchors.status).is_none() {
                say(wire::plain("gone"));
                break;
            }
            let seen = stable::glance(&process, &anchors);
            if let Some(seen) = &seen {
                let said = wire::state(seen);
                if said != last_state {
                    say(said.clone());
                    last_state = said;
                }
            }
            if let Some(take) = recorder.poll(&process, &anchors) {
                if take.frames.len() >= FRAMES_LEAST {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
                    say(wire::kept(&take, &osr::file_name(&take, player, now), &osr::write(&take, player, now)));
                }
            }
            if let (Some(take), Some(seen)) = (recorder.recording(), &seen) {
                if told_at.elapsed() >= Duration::from_secs(1) {
                    say(wire::playing(take, seen.time_ms));
                    told_at = Instant::now();
                    alive_at = Instant::now();
                }
            }
            if alive_at.elapsed() >= Duration::from_secs(5) {
                say(wire::plain("alive"));
                alive_at = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(windows)]
const FRAMES_LEAST: usize = 120;

#[cfg(windows)]
fn record(process: &dossier_witness::windows::Process, anchors: &dossier_witness::stable::Anchors, out: &std::path::Path, player: &str) {
    use dossier_witness::{osr, stable};
    let _ = std::fs::create_dir_all(out);
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
                let file = out.join(osr::file_name(&take, player, now));
                match std::fs::write(&file, osr::write(&take, player, now)) {
                    Ok(()) => println!("witness: kept {} ({} frames, {} points, passed {})", file.display(), take.frames.len(), take.play.score, take.passed),
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
