use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

static BUNDLED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/witness.exe"));

const CLIENT: &str = "osu!.exe";
const LOOK_EVERY: Duration = Duration::from_secs(4);
const AGAIN_AFTER: Duration = Duration::from_secs(3);
const NAME_MOST: usize = 160;
const LEASH: &str = "witness.alive";
const LEASH_EVERY: Duration = Duration::from_secs(5);
const TOLD_LEAST: u32 = 30;
const NOT_PLAYED: u32 = 2048 | 4_194_304;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MapFacts {
    pub id: i64,
    pub set: i64,
    pub status: String,
    pub stars: f64,
    pub base_stars: f64,
    pub ar: f64,
    pub cs: f64,
    pub od: f64,
    pub hp: f64,
    pub bpm: f64,
    pub length: i64,
    pub objects: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct State {
    pub mode: String,
    pub md5: String,
    pub id: i64,
    pub set: i64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: String,
    pub facts: Option<MapFacts>,
}

impl State {
    pub fn map_line(&self) -> String {
        match (self.artist.is_empty(), self.version.is_empty()) {
            (false, false) => format!("{} — {} [{}]", self.artist, self.title, self.version),
            (false, true) => format!("{} — {}", self.artist, self.title),
            (true, false) => format!("{} [{}]", self.title, self.version),
            (true, true) => self.title.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub md5: String,
    pub time: i64,
    pub frames: u64,
    pub score: i64,
    pub combo: u32,
    pub max_combo: u32,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub miss: u32,
    pub mods: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub name: String,
    pub passed: bool,
    pub failed: bool,
    pub md5: String,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub score: i64,
    pub frames: u64,
    pub osr: String,
    pub id: i64,
    pub set: i64,
    pub creator: String,
    pub watched: Option<bool>,
    pub facts: Option<MapFacts>,
}

const SITTING_MOST: i64 = 24 * 3600;
const SITTING_TELL_EVERY: i64 = 300;
const PLAY_TICK_MOST: i64 = 5;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Sitting {
    pub time_zone: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub play_seconds: i64,
    pub plays: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sittings {
    open: Option<Sitting>,
    told_at: i64,
    ticked_at: Option<i64>,
}

impl Sittings {
    pub fn current(&self) -> Option<&Sitting> {
        self.open.as_ref()
    }

    pub fn take(&mut self, event: &Event, now: i64, zone: &str) -> Option<Sitting> {
        match event {
            Event::Attached { .. } => {
                let closed = self.close(now);
                self.begin(now, zone);
                closed.or_else(|| self.tell(now))
            }
            Event::Waiting | Event::Gone | Event::Absent | Event::Unavailable => self.close(now),
            Event::Playing(_) => {
                self.begin_if_none(now, zone);
                if let (Some(sitting), Some(before)) = (self.open.as_mut(), self.ticked_at) {
                    sitting.play_seconds += (now - before).clamp(0, PLAY_TICK_MOST);
                }
                self.ticked_at = Some(now);
                self.moved(now)
            }
            Event::Kept(_) => {
                self.begin_if_none(now, zone);
                if let Some(sitting) = self.open.as_mut() {
                    sitting.plays += 1;
                }
                self.ticked_at = None;
                self.moved(now)
            }
            Event::State(state) => {
                if state.mode != "Play" {
                    self.ticked_at = None;
                }
                self.begin_if_none(now, zone);
                self.moved(now)
            }
            Event::Loading | Event::Alive => None,
        }
    }

    fn begin(&mut self, now: i64, zone: &str) {
        self.open = Some(Sitting { time_zone: zone.to_owned(), started_at: now, ended_at: now, ..Sitting::default() });
        self.told_at = 0;
        self.ticked_at = None;
    }

    fn begin_if_none(&mut self, now: i64, zone: &str) {
        if self.open.is_none() {
            self.begin(now, zone);
        }
    }

    fn moved(&mut self, now: i64) -> Option<Sitting> {
        let sitting = self.open.as_mut()?;
        sitting.ended_at = now;
        if now - sitting.started_at >= SITTING_MOST {
            let done = self.open.take();
            self.ticked_at = None;
            return done;
        }
        self.tell(now)
    }

    fn tell(&mut self, now: i64) -> Option<Sitting> {
        if self.told_at != 0 && now - self.told_at < SITTING_TELL_EVERY {
            return None;
        }
        self.told_at = now;
        self.open.clone()
    }

    fn close(&mut self, now: i64) -> Option<Sitting> {
        self.ticked_at = None;
        let mut sitting = self.open.take()?;
        sitting.ended_at = now.max(sitting.started_at);
        Some(sitting)
    }
}

pub fn time_zone() -> String {
    iana_time_zone::get_timezone().unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Told {
    pub md5: String,
    pub replay: String,
    pub id: i64,
    pub set: i64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: String,
    pub mods: u32,
    pub score: i64,
    pub max_combo: u32,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub geki: u32,
    pub katu: u32,
    pub miss: u32,
    pub passed: bool,
    pub ended: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facts: Option<MapFacts>,
}

pub fn told(kept: &Kept, own: &str, now: i64) -> Option<Told> {
    if kept.watched != Some(false) || own.trim().is_empty() {
        return None;
    }
    let bytes = bytes_of(&kept.osr)?;
    let replay = dossier_replay::Replay::heading(&bytes).ok()?;
    let mods = replay.mods.raw();
    let player = replay.player.trim().to_lowercase();
    if replay.mode != dossier_replay::GameMode::Standard || mods & NOT_PLAYED != 0 || (!player.is_empty() && player != own.trim().to_lowercase()) {
        return None;
    }
    if !kept.passed && replay.hits.total_hits() < TOLD_LEAST {
        return None;
    }
    Some(Told {
        md5: replay.beatmap_hash.to_lowercase(),
        replay: replay.replay_hash.to_lowercase(),
        id: kept.id.max(0),
        set: kept.set.max(0),
        artist: kept.artist.clone(),
        title: kept.title.clone(),
        version: kept.version.clone(),
        creator: kept.creator.clone(),
        mods,
        score: i64::from(replay.score.max(0)),
        max_combo: u32::from(replay.max_combo),
        n300: u32::from(replay.hits.count_300),
        n100: u32::from(replay.hits.count_100),
        n50: u32::from(replay.hits.count_50),
        geki: u32::from(replay.hits.count_geki),
        katu: u32::from(replay.hits.count_katu),
        miss: u32::from(replay.hits.count_miss),
        passed: kept.passed,
        ended: now,
        facts: kept.facts.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    Waiting,
    Alive,
    Loading,
    Gone,
    Attached {
        #[serde(default)]
        pid: u32,
        #[serde(default)]
        build: String,
        #[serde(default)]
        player: String,
    },
    State(State),
    Playing(Progress),
    Kept(Kept),
    #[serde(skip)]
    Absent,
    #[serde(skip)]
    Unavailable,
}

pub fn read(line: &str) -> Option<Event> {
    serde_json::from_str(line.trim()).ok()
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Status {
    #[default]
    Off,
    Unavailable,
    Absent,
    Loading,
    Watching,
    Playing,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Seen {
    pub status: Status,
    pub state: Option<State>,
    pub playing: Option<Progress>,
    pub kept: u32,
    pub written: u32,
    pub told: u32,
    pub pending: u32,
    pub untold: bool,
    pub build: String,
    pub player: String,
    pub history_told: u32,
    pub history_failed: bool,
}

impl Seen {
    pub fn take(&mut self, event: &Event) {
        match event {
            Event::Unavailable => *self = Seen { status: Status::Unavailable, state: None, playing: None, build: String::new(), player: String::new(), ..self.clone() },
            Event::Waiting | Event::Gone | Event::Absent => *self = Seen { status: Status::Absent, state: None, playing: None, build: String::new(), player: String::new(), ..self.clone() },
            Event::Attached { build, player, .. } => {
                self.status = Status::Loading;
                self.state = None;
                self.playing = None;
                self.build = build.clone();
                self.player = player.clone();
            }
            Event::Loading => {
                self.status = Status::Loading;
                self.state = None;
                self.playing = None;
            }
            Event::State(state) => {
                if state.mode != "Play" {
                    self.playing = None;
                }
                self.status = if self.playing.is_some() { Status::Playing } else { Status::Watching };
                self.state = Some(state.clone());
            }
            Event::Playing(progress) => {
                self.status = Status::Playing;
                self.playing = Some(progress.clone());
            }
            Event::Kept(_) => {
                self.kept += 1;
                self.playing = None;
                if self.status == Status::Playing {
                    self.status = Status::Watching;
                }
            }
            Event::Alive => {}
        }
    }
}

pub fn bundled() -> bool {
    !BUNDLED.is_empty()
}

pub fn program() -> Option<PathBuf> {
    if let Some(given) = std::env::var_os("DOSSIER_WITNESS").map(PathBuf::from).filter(|path| path.is_file()) {
        return Some(given);
    }
    if !bundled() {
        return None;
    }
    let dir = crate::sources::own_root().join("bin");
    let path = dir.join("witness.exe");
    if std::fs::read(&path).is_ok_and(|held| held == BUNDLED) {
        return Some(path);
    }
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(&path, BUNDLED).ok()?;
    Some(path)
}

pub fn folder() -> PathBuf {
    crate::sources::witnessed_root()
}

pub fn bytes_of(hex: &str) -> Option<Vec<u8>> {
    let raw = hex.as_bytes();
    if raw.len() % 2 != 0 {
        return None;
    }
    let digit = |byte: u8| (byte as char).to_digit(16).map(|value| value as u8);
    raw.chunks_exact(2).map(|pair| Some(digit(pair[0])? << 4 | digit(pair[1])?)).collect()
}

fn clean(name: &str) -> String {
    let stem = name.strip_suffix(".osr").unwrap_or(name);
    let kept: String = stem.chars().map(|c| if c.is_alphanumeric() || " -_[]().,!'".contains(c) { c } else { '_' }).collect();
    let kept = kept.trim().trim_matches('.').chars().take(NAME_MOST).collect::<String>();
    if kept.is_empty() { "play".to_owned() } else { kept }
}

pub fn keep(kept: &Kept, folder: &Path) -> Result<PathBuf, String> {
    let bytes = bytes_of(&kept.osr).filter(|bytes| !bytes.is_empty()).ok_or_else(|| "the replay did not arrive whole".to_owned())?;
    dossier_replay::Replay::heading(&bytes).map_err(|_| "what arrived is not a replay".to_owned())?;
    std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    let stem = clean(&kept.name);
    let mut path = folder.join(format!("{stem}.osr"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("{stem} ({n}).osr"));
        n += 1;
    }
    std::fs::write(&path, &bytes).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    Direct,
    CrossOver { wine: PathBuf, bottle: String },
    Wine { loader: PathBuf, prefix: PathBuf },
}

pub fn client_pid(listed: &str) -> Option<u32> {
    listed.lines().find_map(|line| {
        let line = line.trim();
        let (pid, command) = line.split_once(char::is_whitespace)?;
        let command = command.trim();
        (command.to_lowercase().contains(&CLIENT.to_lowercase()) && !command.contains("witness")).then(|| pid.parse().ok()).flatten()
    })
}

pub fn prefix_of(inside: &Path) -> Option<PathBuf> {
    let mut found = None;
    let mut walked = PathBuf::new();
    for part in inside.components() {
        if part.as_os_str() == "drive_c" {
            found = Some(walked.clone());
        }
        walked.push(part);
    }
    found.filter(|prefix| prefix.parent().is_some())
}

pub fn wine_root(mapped: &[String]) -> Option<PathBuf> {
    mapped.iter().find_map(|path| path.find("/lib/wine/").map(|at| PathBuf::from(&path[..at])))
}

pub fn launch_of(cwd: &Path, mapped: &[String], said_loader: Option<&Path>) -> Option<Launch> {
    let prefix = prefix_of(cwd)?;
    let root = wine_root(mapped);
    let in_bottles = prefix.parent().and_then(Path::file_name).is_some_and(|name| name == "Bottles");
    if in_bottles {
        let wine = root.as_ref().map(|root| root.join("bin").join("wine")).filter(|wine| wine.is_file()).or_else(|| Some(PathBuf::from("/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine")).filter(|wine| wine.is_file()));
        if let (Some(wine), Some(bottle)) = (wine, prefix.file_name().and_then(|name| name.to_str())) {
            return Some(Launch::CrossOver { wine, bottle: bottle.to_owned() });
        }
    }
    let loader = said_loader
        .map(Path::to_path_buf)
        .filter(|loader| loader.is_file())
        .or_else(|| root.as_ref().and_then(|root| ["wine64", "wine"].iter().map(|name| root.join("bin").join(name)).find(|candidate| candidate.is_file())))
        .unwrap_or_else(|| PathBuf::from("wine"));
    Some(Launch::Wine { loader, prefix })
}

#[cfg(windows)]
pub fn launch() -> Option<Launch> {
    Some(Launch::Direct)
}

#[cfg(target_os = "macos")]
pub fn launch() -> Option<Launch> {
    let said = |program: &str, args: &[&str]| Command::new(program).args(args).stderr(Stdio::null()).output().ok().map(|out| String::from_utf8_lossy(&out.stdout).into_owned());
    let pid = client_pid(&said("/bin/ps", &["-axo", "pid=,command="])?)?.to_string();
    let named = |listed: String| -> Vec<String> { listed.lines().filter_map(|line| line.strip_prefix('n')).map(str::to_owned).collect() };
    let cwd = named(said("/usr/sbin/lsof", &["-a", "-p", &pid, "-d", "cwd", "-Fn"])?).into_iter().next()?;
    let mapped = named(said("/usr/sbin/lsof", &["-a", "-p", &pid, "-d", "txt", "-Fn"]).unwrap_or_default());
    launch_of(Path::new(&cwd), &mapped, None)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn launch() -> Option<Launch> {
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let dir = entry.path();
        if !entry.file_name().to_str().is_some_and(|name| name.bytes().all(|b| b.is_ascii_digit())) {
            continue;
        }
        let Ok(command) = std::fs::read(dir.join("cmdline")) else {
            continue;
        };
        let command = String::from_utf8_lossy(&command).replace('\0', " ");
        if client_pid(&format!("1 {command}")).is_none() {
            continue;
        }
        let environ = std::fs::read(dir.join("environ")).unwrap_or_default();
        let named = |key: &str| environ.split(|byte| *byte == 0).filter_map(|pair| std::str::from_utf8(pair).ok()).find_map(|pair| pair.strip_prefix(key).map(PathBuf::from));
        let loader = named("WINELOADER=");
        if let Some(prefix) = named("WINEPREFIX=").filter(|prefix| prefix.join("drive_c").is_dir()) {
            return Some(Launch::Wine { loader: loader.filter(|loader| loader.is_file()).unwrap_or_else(|| PathBuf::from("wine")), prefix });
        }
        if let Some(found) = std::fs::read_link(dir.join("cwd")).ok().and_then(|cwd| launch_of(&cwd, &[], loader.as_deref())) {
            return Some(found);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Overlay {
    pub english: bool,
    pub keeps: bool,
}

pub fn command(launch: &Launch, program: &Path, player: &str, overlay: Option<Overlay>) -> Command {
    let mut command = match launch {
        Launch::Direct => Command::new(program),
        Launch::CrossOver { wine, bottle } => {
            let mut command = Command::new(wine);
            command.arg("--bottle").arg(bottle).arg(program);
            command
        }
        Launch::Wine { loader, prefix } => {
            let mut command = Command::new(loader);
            command.env("WINEPREFIX", prefix).env("WINEDEBUG", "-all").arg(program);
            command
        }
    };
    command.arg("--serve").arg("--leash");
    if !player.is_empty() {
        command.arg("--player").arg(player);
    }
    if let Some(overlay) = overlay {
        command.arg("--hud");
        if overlay.english {
            command.arg("--lang").arg("en");
        }
        if overlay.keeps {
            command.arg("--keeps");
        }
    }
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

#[derive(Debug, Default)]
pub struct Control {
    stop: AtomicBool,
    child: Mutex<Option<Child>>,
    overlay: Mutex<Option<Overlay>>,
    context: Mutex<Option<dossier_hud::context::Packet>>,
    pid: std::sync::atomic::AtomicU32,
}

pub fn context_path() -> PathBuf { crate::sources::own_root().join("witness-context.json") }

impl Control {
    pub fn context(&self, event: &Event, state: Option<&State>, pools: &[crate::pools::Pool], english: bool) {
        if let Event::Attached { pid, .. } = event { self.pid.store(*pid, Ordering::SeqCst); }
        let absent = matches!(event, Event::Gone | Event::Waiting | Event::Absent | Event::Unavailable);
        if absent { self.pid.store(0, Ordering::SeqCst); }
        let pid = self.pid.load(Ordering::SeqCst);
        let mut held = self.context.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *held = (pid != 0 && !matches!(event, Event::Loading)).then(|| {
            let map_md5 = state.map(|state| state.md5.clone()).filter(|hash| !hash.is_empty());
            let card = map_md5.as_ref().and_then(|hash| pools.iter().filter(|pool| !pool.collection).find_map(|pool| {
                pool.slots.iter().find(|slot| slot.hash.as_ref().is_some_and(|known| known.eq_ignore_ascii_case(hash))).map(|slot| dossier_hud::Card {
                    pool: Some(pool.name.clone()), stars: slot.measure.as_ref().filter(|_| slot.mods == crate::pools::Mod::Nm).map(|measure| measure.stars), places: Vec::new(),
                })
            }));
            dossier_hud::context::Packet { version: 1, pid, at_ms: 0, lang: if english { dossier_hud::Lang::En } else { dossier_hud::Lang::Ru }, map_md5, card, day: None }
        });
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(mut child) = self.child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn overlay(&self) -> Option<Overlay> {
        *self.overlay.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn show(&self, overlay: Option<Overlay>) -> bool {
        let mut held = self.overlay.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *held == overlay {
            return false;
        }
        *held = overlay;
        drop(held);
        if let Some(mut child) = self.child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        true
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    fn rest(&self, time: Duration) {
        let step = Duration::from_millis(200);
        let mut left = time;
        while !self.stopped() && left > Duration::ZERO {
            std::thread::sleep(step.min(left));
            left = left.saturating_sub(step);
        }
    }
}

pub fn follow(lines: impl BufRead, control: &Control, push: &mut dyn FnMut(Event) -> bool) -> bool {
    for line in lines.lines() {
        let Ok(line) = line else {
            return true;
        };
        if control.stopped() {
            return false;
        }
        if let Some(event) = read(&line) {
            if !matches!(event, Event::Alive) && !push(event) {
                return false;
            }
        }
    }
    true
}

pub fn run(control: Arc<Control>, player: String, push: &mut dyn FnMut(Event) -> bool) {
    let Some(program) = program() else {
        let _ = push(Event::Unavailable);
        return;
    };
    let leash = program.parent().map(|dir| dir.join(LEASH));
    if let Some(leash) = leash.clone() {
        let _ = std::fs::write(&leash, b"held");
        let holder = control.clone();
        std::thread::spawn(move || {
            while !holder.stopped() {
                let _ = std::fs::write(&leash, b"held");
                let packet = holder.context.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
                if let Some(mut packet) = packet {
                    packet.at_ms = dossier_hud::context::now_ms();
                    let _ = dossier_hud::context::write(&context_path(), &packet);
                } else { let _ = std::fs::remove_file(context_path()); }
                holder.rest(LEASH_EVERY);
            }
            let _ = std::fs::remove_file(&leash);
            let _ = std::fs::remove_file(context_path());
        });
    }
    let mut absent_told = false;
    while !control.stopped() {
        let Some(launch) = launch() else {
            if !absent_told && !push(Event::Absent) {
                return;
            }
            absent_told = true;
            control.rest(LOOK_EVERY);
            continue;
        };
        absent_told = false;
        let Ok(mut child) = command(&launch, &program, &player, control.overlay()).spawn() else {
            if !push(Event::Unavailable) {
                return;
            }
            control.rest(LOOK_EVERY * 4);
            continue;
        };
        let out = child.stdout.take();
        *control.child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(child);
        let going = match out {
            Some(out) => follow(std::io::BufReader::new(out), &control, push),
            None => true,
        };
        if let Some(mut child) = control.child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if !going {
            return;
        }
        control.rest(AGAIN_AFTER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_context_tracks_attachment_loading_and_disconnect() {
        let control = Control::default();
        let attached = Event::Attached { pid: 42, build: String::new(), player: String::new() };
        control.context(&attached, None, &[], true);
        assert_eq!(control.context.lock().unwrap().as_ref().unwrap().pid, 42);
        control.context(&Event::Loading, None, &[], true);
        assert!(control.context.lock().unwrap().is_none());
        let state = State { md5: "a".repeat(32), ..State::default() };
        control.context(&Event::State(state.clone()), Some(&state), &[], true);
        let packet = control.context.lock().unwrap().clone().unwrap();
        assert_eq!(packet.pid, 42);
        assert_eq!(packet.map_md5, Some("a".repeat(32)));
        assert_eq!(packet.lang, dossier_hud::Lang::En);
        assert!(packet.day.is_none() && packet.card.is_none());
        control.context(&Event::Gone, None, &[], true);
        assert!(control.context.lock().unwrap().is_none());
    }

    #[test]
    fn overlay_pool_card_matches_the_map_and_excludes_collections() {
        let control = Control::default();
        let mut pool = crate::pools::Pool::new(crate::pools::Frame::Free, "Cup", 0);
        let mut slot = crate::pools::Slot::empty(crate::pools::Mod::Nm);
        slot.hash = Some("a".repeat(32));
        pool.slots.push(slot);
        let state = State { md5: "A".repeat(32), ..State::default() };
        let attached = Event::Attached { pid: 42, build: String::new(), player: String::new() };
        control.context(&attached, Some(&state), &[pool.clone()], false);
        assert_eq!(control.context.lock().unwrap().as_ref().unwrap().card.as_ref().unwrap().pool.as_deref(), Some("Cup"));
        pool.collection = true;
        control.context(&Event::State(state.clone()), Some(&state), &[pool], false);
        assert!(control.context.lock().unwrap().as_ref().unwrap().card.is_none());
    }

    #[test]
    fn what_witness_says_is_read_line_by_line() {
        assert_eq!(read(r#"{"event":"waiting"}"#), Some(Event::Waiting));
        assert_eq!(read(r#"{"event":"attached","pid":436}"#), Some(Event::Attached { pid: 436, build: String::new(), player: String::new() }));
        let state = read(r#"{"event":"state","mode":"SelectPlay","md5":"0123456789abcdef0123456789abcdef","id":7,"set":8,"artist":"xi","title":"FREEDOM \"DiVE\"","version":"FOUR","creator":"N"}"#);
        let Some(Event::State(state)) = state else {
            panic!("not a state: {state:?}");
        };
        assert_eq!((state.mode.as_str(), state.id, state.map_line().as_str()), ("SelectPlay", 7, "xi — FREEDOM \"DiVE\" [FOUR]"));
        assert_eq!(read(r#"{"event":"state","mode":"Menu"}"#), Some(Event::State(State { mode: "Menu".into(), ..State::default() })));
        let playing = read(r#"{"event":"playing","md5":"ab","time":61500,"frames":3886,"score":92242,"combo":17,"max_combo":81,"n300":81,"n100":12,"n50":0,"miss":1,"mods":24}"#);
        assert!(matches!(playing, Some(Event::Playing(Progress { frames: 3886, score: 92_242, miss: 1, mods: 24, .. }))));
        assert_eq!(read("witness: something else"), None);
        assert_eq!(read(r#"{"event":"a new thing"}"#), None);
    }

    fn playing() -> Event {
        Event::Playing(Progress::default())
    }

    fn state(mode: &str) -> Event {
        Event::State(State { mode: mode.into(), ..State::default() })
    }

    #[test]
    fn a_session_is_told_when_the_client_is_found_and_again_when_it_goes() {
        let mut sittings = Sittings::default();
        let zone = "Europe/Moscow";
        let began = sittings.take(&Event::Attached { pid: 1, build: String::new(), player: String::new() }, 1_000, zone).expect("the beginning is told");
        assert_eq!((began.started_at, began.ended_at, began.play_seconds, began.plays, began.time_zone.as_str()), (1_000, 1_000, 0, 0, zone));
        assert!(sittings.take(&Event::Alive, 1_001, zone).is_none() && sittings.take(&Event::Loading, 1_002, zone).is_none());
        let ended = sittings.take(&Event::Gone, 2_000, zone).expect("the end is told");
        assert_eq!((ended.started_at, ended.ended_at), (1_000, 2_000));
        assert!(sittings.current().is_none());
        assert!(sittings.take(&Event::Gone, 2_001, zone).is_none(), "nothing is told twice");
    }

    #[test]
    fn only_the_time_spent_in_a_play_counts_as_play_time_and_a_gap_is_not_counted_whole() {
        let mut sittings = Sittings::default();
        sittings.take(&Event::Attached { pid: 1, build: String::new(), player: String::new() }, 1_000, "UTC");
        sittings.take(&state("SelectPlay"), 1_010, "UTC");
        for second in 1_020..1_030 {
            sittings.take(&playing(), second, "UTC");
        }
        assert_eq!(sittings.current().map(|s| s.play_seconds), Some(9));
        sittings.take(&playing(), 1_500, "UTC");
        assert_eq!(sittings.current().map(|s| s.play_seconds), Some(14), "a silence of minutes adds at most a few seconds");
        sittings.take(&state("SelectPlay"), 1_501, "UTC");
        sittings.take(&playing(), 1_600, "UTC");
        assert_eq!(sittings.current().map(|s| s.play_seconds), Some(14), "leaving the play and coming back does not bridge the gap");
        sittings.take(&Event::Kept(Kept::default()), 1_601, "UTC");
        sittings.take(&Event::Kept(Kept::default()), 1_602, "UTC");
        assert_eq!(sittings.current().map(|s| s.plays), Some(2));
    }

    #[test]
    fn a_running_session_is_told_again_only_every_five_minutes() {
        let mut sittings = Sittings::default();
        sittings.take(&Event::Attached { pid: 1, build: String::new(), player: String::new() }, 1_000, "UTC");
        let mut told = 0;
        for second in 1_001..1_700 {
            if sittings.take(&playing(), second, "UTC").is_some() {
                told += 1;
            }
        }
        assert_eq!(told, 2, "once after five minutes and once more after ten");
    }

    #[test]
    fn a_session_that_has_gone_on_for_a_day_is_closed_and_the_next_one_begins_afresh() {
        let mut sittings = Sittings::default();
        sittings.take(&Event::Attached { pid: 1, build: String::new(), player: String::new() }, 1_000, "UTC");
        let long = sittings.take(&playing(), 1_000 + SITTING_MOST, "UTC").expect("the day-long one is handed over");
        assert_eq!(long.ended_at - long.started_at, SITTING_MOST);
        assert!(sittings.current().is_none());
        let next = sittings.take(&playing(), 1_000 + SITTING_MOST + 1, "UTC").expect("a new one is told at once");
        assert_eq!(next.started_at, 1_000 + SITTING_MOST + 1);
    }

    #[test]
    fn what_the_client_knows_of_the_map_comes_with_the_state_and_the_kept_play_and_goes_on_to_the_bot() {
        let facts = r#""facts":{"id":7,"set":8,"status":"ranked","stars":6.25,"base_stars":4.5,"ar":9,"cs":4,"od":8.5,"hp":5,"bpm":180,"length":120,"objects":300}"#;
        let Some(Event::State(state)) = read(&format!(r#"{{"event":"state","mode":"SelectPlay","md5":"ab","id":7,"set":8,{facts}}}"#)) else {
            panic!("a state is read");
        };
        assert_eq!(state.facts.as_ref().map(|facts| (facts.status.as_str(), facts.stars, facts.bpm, facts.length)), Some(("ranked", 6.25, 180.0, 120)));
        assert!(matches!(read(r#"{"event":"state","mode":"Menu"}"#), Some(Event::State(State { facts: None, .. }))), "a state without them is still read");
        let replay = include_bytes!("../tests/fixtures/witness.osr");
        let hex: String = replay.iter().map(|byte| format!("{byte:02x}")).collect();
        let Some(Event::Kept(kept)) = read(&format!(r#"{{"event":"kept","passed":true,"watched":false,"osr":"{hex}",{facts}}}"#)) else {
            panic!("a kept play is read");
        };
        let own = dossier_replay::Replay::heading(replay).expect("a replay").player;
        let told = told(&kept, &own, 1_000).expect("a play of one's own is told");
        assert_eq!(told.facts.as_ref().map(|facts| facts.stars), Some(6.25));
        let said = serde_json::to_value(&told).expect("it says itself");
        assert_eq!(said["facts"]["bpm"], 180.0);
        let bare = serde_json::to_value(Told { facts: None, ..told }).expect("it says itself");
        assert!(bare.get("facts").is_none());
    }

    #[test]
    fn a_session_is_told_to_the_bot_in_the_words_the_bot_reads() {
        let said = serde_json::to_value(Sitting { time_zone: "Europe/Moscow".into(), started_at: 10, ended_at: 20, play_seconds: 5, plays: 2 }).expect("a session says itself");
        assert_eq!(said, serde_json::json!({"time_zone": "Europe/Moscow", "started_at": 10, "ended_at": 20, "play_seconds": 5, "plays": 2}));
    }

    #[test]
    fn a_client_met_in_the_middle_of_a_play_still_makes_a_session() {
        let mut sittings = Sittings::default();
        let first = sittings.take(&playing(), 500, "Asia/Tokyo").expect("begun by the first sign of play");
        assert_eq!((first.started_at, first.time_zone.as_str()), (500, "Asia/Tokyo"));
    }

    #[test]
    fn the_client_tells_its_build_and_its_player_when_it_is_found() {
        let mut seen = Seen::default();
        seen.take(&read(r#"{"event":"attached","pid":436,"build":"b20260924cuttingedge","player":"NaumRedlo"}"#).expect("read"));
        assert_eq!((seen.status.clone(), seen.build.as_str(), seen.player.as_str()), (Status::Loading, "b20260924cuttingedge", "NaumRedlo"));
        seen.take(&read(r#"{"event":"loading"}"#).expect("read"));
        assert_eq!(seen.build, "b20260924cuttingedge", "the build is kept while the same client loads");
        seen.take(&read(r#"{"event":"gone"}"#).expect("read"));
        assert!(seen.build.is_empty() && seen.player.is_empty(), "and forgotten with the client");
        seen.take(&read(r#"{"event":"attached","pid":7}"#).expect("an older Witness says less"));
        assert!(seen.build.is_empty());
        let kept = read(r#"{"event":"kept","name":"a.osr","passed":false,"failed":true,"md5":"ab","osr":"00"}"#);
        assert!(matches!(kept, Some(Event::Kept(Kept { failed: true, passed: false, .. }))));
    }

    #[test]
    fn a_kept_play_becomes_a_replay_file_and_nothing_else_does() {
        let dir = std::env::temp_dir().join(format!("dossier-witness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let replay = include_bytes!("../tests/fixtures/witness.osr");
        let hex: String = replay.iter().map(|byte| format!("{byte:02x}")).collect();
        let kept = Kept { name: "NaumRedlo - xi - FREEDOM: DiVE [FOUR] (7).osr".into(), osr: hex, passed: true, ..Kept::default() };
        let first = keep(&kept, &dir).expect("kept");
        assert_eq!(first.file_name().and_then(|name| name.to_str()), Some("NaumRedlo - xi - FREEDOM_ DiVE [FOUR] (7).osr"));
        assert_eq!(std::fs::read(&first).unwrap(), replay);
        let second = keep(&kept, &dir).expect("kept again");
        assert!(second != first && second.to_string_lossy().ends_with("(7) (2).osr"));
        let outside = Kept { name: "../../etc/passwd".into(), ..kept.clone() };
        assert!(keep(&outside, &dir).expect("kept inside").starts_with(&dir));
        assert!(keep(&Kept { osr: "zz".into(), ..kept.clone() }, &dir).is_err());
        assert!(keep(&Kept { osr: "00010203".into(), ..kept.clone() }, &dir).is_err(), "bytes that are not a replay were kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_a_play_that_was_really_played_by_the_person_is_told() {
        let replay = include_bytes!("../tests/fixtures/witness.osr");
        let heading = dossier_replay::Replay::heading(replay).expect("a replay");
        let hex: String = replay.iter().map(|byte| format!("{byte:02x}")).collect();
        let kept = Kept { osr: hex, passed: true, id: 129_891, set: 39_804, artist: "xi".into(), title: "FREEDOM DiVE".into(), version: "FOUR DIMENSIONS".into(), creator: "Nakagawa-Kanon".into(), watched: Some(false), ..Kept::default() };
        let own = heading.player.to_uppercase();
        let play = told(&kept, &own, 1_790_000_000).expect("a play to tell");
        assert_eq!((play.md5.as_str(), play.replay.as_str()), (heading.beatmap_hash.as_str(), heading.replay_hash.as_str()));
        assert_eq!((play.n300, play.n100, play.n50, play.miss), (u32::from(heading.hits.count_300), u32::from(heading.hits.count_100), u32::from(heading.hits.count_50), u32::from(heading.hits.count_miss)));
        assert_eq!((play.id, play.set, play.passed, play.ended, play.mods), (129_891, 39_804, true, 1_790_000_000, heading.mods.raw()));
        let said = serde_json::to_value(&play).expect("json");
        assert_eq!((said["max_combo"].as_u64(), said["creator"].as_str()), (Some(u64::from(heading.max_combo)), Some("Nakagawa-Kanon")));
        assert_eq!(told(&Kept { watched: Some(true), ..kept.clone() }, &own, 0), None, "a watched replay was told as a play");
        assert_eq!(told(&Kept { watched: None, ..kept.clone() }, &own, 0), None, "a play nobody can vouch for was told");
        assert_eq!(told(&kept, "someone else", 0), None, "another player's replay was told");
        assert_eq!(told(&kept, " ", 0), None);
        assert_eq!(told(&Kept { osr: "00".into(), ..kept.clone() }, &own, 0), None);
        let left = told(&Kept { passed: false, ..kept.clone() }, &own, 0);
        assert_eq!(left.is_some(), heading.hits.total_hits() >= TOLD_LEAST);
        let event = read(r#"{"event":"kept","name":"a.osr","passed":true,"md5":"ab","id":7,"set":8,"artist":"xi","title":"t","version":"v","creator":"N","score":5,"frames":300,"watched":false,"osr":"00"}"#);
        assert!(matches!(event, Some(Event::Kept(Kept { id: 7, set: 8, watched: Some(false), .. }))));
        let old = read(r#"{"event":"kept","name":"a.osr","passed":true,"md5":"ab","artist":"xi","title":"t","version":"v","score":5,"frames":300,"osr":"00"}"#);
        assert!(matches!(old, Some(Event::Kept(Kept { id: 0, watched: None, .. }))));
    }

    #[test]
    fn the_running_client_tells_where_its_wine_lives() {
        let listed = "  501 /usr/bin/something\n10625 C:\\users\\crossover\\AppData\\Local\\osu!\\osu!.exe \n10700 Z:\\tmp\\witness.exe --process osu!.exe\n";
        assert_eq!(client_pid(listed), Some(10625));
        assert_eq!(client_pid("12 /bin/zsh\n"), None);
        let cwd = Path::new("/Users/none/Library/Application Support/CrossOver/Bottles/osu-stable/drive_c/users/crossover/AppData/Local/osu!");
        assert_eq!(prefix_of(cwd), Some(PathBuf::from("/Users/none/Library/Application Support/CrossOver/Bottles/osu-stable")));
        assert_eq!(prefix_of(Path::new("/home/none/Games/osu")), None);
        let mapped = vec!["/opt/wine-osu/lib/wine/x86_64-unix/ntdll.so".to_owned()];
        assert_eq!(wine_root(&mapped), Some(PathBuf::from("/opt/wine-osu")));
        let plain = launch_of(Path::new("/home/none/.local/share/osu-wine/WINE.win32/drive_c/osu"), &mapped, None);
        assert_eq!(plain, Some(Launch::Wine { loader: PathBuf::from("wine"), prefix: PathBuf::from("/home/none/.local/share/osu-wine/WINE.win32") }));
        assert_eq!(launch_of(Path::new("/home/none/osu"), &mapped, None), None);
    }

    #[test]
    fn the_overlay_is_asked_for_only_when_it_is_switched_on_and_a_change_restarts_witness() {
        let program = Path::new("/home/none/.dossier/bin/witness.exe");
        let args = |overlay: Option<Overlay>| command(&Launch::Direct, program, "", overlay).get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(args(None), ["--serve", "--leash"]);
        assert_eq!(args(Some(Overlay::default())), ["--serve", "--leash", "--hud"]);
        assert_eq!(args(Some(Overlay { english: true, keeps: true })), ["--serve", "--leash", "--hud", "--lang", "en", "--keeps"]);
        let control = Control::default();
        assert_eq!(control.overlay(), None);
        assert!(control.show(Some(Overlay::default())), "a change is told so that the program is started again");
        assert!(!control.show(Some(Overlay::default())), "the same word twice changes nothing");
        assert_eq!(control.overlay(), Some(Overlay::default()));
        assert!(control.show(None));
    }

    #[test]
    fn witness_is_started_inside_the_client_s_own_prefix() {
        let program = Path::new("/home/none/.dossier/bin/witness.exe");
        let args = |command: &Command| command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>();
        let direct = command(&Launch::Direct, program, "NaumRedlo", None);
        assert_eq!((direct.get_program(), args(&direct)), (program.as_os_str(), vec!["--serve".to_owned(), "--leash".to_owned(), "--player".to_owned(), "NaumRedlo".to_owned()]));
        let bottle = command(&Launch::CrossOver { wine: PathBuf::from("/cx/bin/wine"), bottle: "osu-stable".into() }, program, "", None);
        assert_eq!(args(&bottle), vec!["--bottle", "osu-stable", "/home/none/.dossier/bin/witness.exe", "--serve", "--leash"]);
        let wine = command(&Launch::Wine { loader: PathBuf::from("/opt/wine/bin/wine"), prefix: PathBuf::from("/home/none/prefix") }, program, "", None);
        assert_eq!(wine.get_program(), "/opt/wine/bin/wine");
        assert!(wine.get_envs().any(|(key, value)| key == "WINEPREFIX" && value == Some(std::ffi::OsStr::new("/home/none/prefix"))));
    }

    #[test]
    fn the_events_are_followed_until_the_application_stops_listening() {
        let said = "{\"event\":\"waiting\"}\n{\"event\":\"alive\"}\nnoise\n{\"event\":\"attached\",\"pid\":1}\n{\"event\":\"state\",\"mode\":\"Play\",\"title\":\"A\"}\n{\"event\":\"playing\",\"frames\":600}\n{\"event\":\"kept\",\"name\":\"a.osr\"}\n{\"event\":\"gone\"}\n";
        let control = Control::default();
        let mut seen = Seen::default();
        let mut statuses = Vec::new();
        let ended = follow(std::io::Cursor::new(said), &control, &mut |event| {
            seen.take(&event);
            statuses.push(seen.status.clone());
            true
        });
        assert!(ended);
        assert_eq!(statuses, vec![Status::Absent, Status::Loading, Status::Watching, Status::Playing, Status::Watching, Status::Absent]);
        assert_eq!(seen.kept, 1);
        let mut heard = 0;
        assert!(!follow(std::io::Cursor::new(said), &control, &mut |_| {
            heard += 1;
            false
        }));
        assert_eq!(heard, 1);
        control.stop();
        assert!(!follow(std::io::Cursor::new(said), &control, &mut |_| true));
    }
}
