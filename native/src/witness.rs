use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

static BUNDLED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/witness.exe"));

const CLIENT: &str = "osu!.exe";
const LOOK_EVERY: Duration = Duration::from_secs(4);
const AGAIN_AFTER: Duration = Duration::from_secs(3);
const NAME_MOST: usize = 160;

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

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub name: String,
    pub passed: bool,
    pub md5: String,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub score: i64,
    pub frames: u64,
    pub osr: String,
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
}

impl Seen {
    pub fn take(&mut self, event: &Event) {
        match event {
            Event::Unavailable => *self = Seen { status: Status::Unavailable, kept: self.kept, ..Seen::default() },
            Event::Waiting | Event::Gone | Event::Absent => *self = Seen { status: Status::Absent, kept: self.kept, ..Seen::default() },
            Event::Attached { .. } | Event::Loading => {
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
    crate::sources::own_root().join("Replays")
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

pub fn command(launch: &Launch, program: &Path, player: &str) -> Command {
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
    command.arg("--serve");
    if !player.is_empty() {
        command.arg("--player").arg(player);
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
}

impl Control {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(mut child) = self.child.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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
        let Ok(mut child) = command(&launch, &program, &player).spawn() else {
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
    fn what_witness_says_is_read_line_by_line() {
        assert_eq!(read(r#"{"event":"waiting"}"#), Some(Event::Waiting));
        assert_eq!(read(r#"{"event":"attached","pid":436}"#), Some(Event::Attached { pid: 436 }));
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
    fn witness_is_started_inside_the_client_s_own_prefix() {
        let program = Path::new("/home/none/.dossier/bin/witness.exe");
        let args = |command: &Command| command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>();
        let direct = command(&Launch::Direct, program, "NaumRedlo");
        assert_eq!((direct.get_program(), args(&direct)), (program.as_os_str(), vec!["--serve".to_owned(), "--player".to_owned(), "NaumRedlo".to_owned()]));
        let bottle = command(&Launch::CrossOver { wine: PathBuf::from("/cx/bin/wine"), bottle: "osu-stable".into() }, program, "");
        assert_eq!(args(&bottle), vec!["--bottle", "osu-stable", "/home/none/.dossier/bin/witness.exe", "--serve"]);
        let wine = command(&Launch::Wine { loader: PathBuf::from("/opt/wine/bin/wine"), prefix: PathBuf::from("/home/none/prefix") }, program, "");
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
