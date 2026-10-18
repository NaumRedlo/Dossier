use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub screen: Option<String>,
    pub map: Option<String>,
}

impl State {
    pub fn line(&mut self, line: &str) {
        if let Some((_, name)) = line.split_once("Game-wide working beatmap updated to ") {
            let name = name.trim();
            self.map = (!name.is_empty()).then(|| name.chars().take(300).collect());
        }
        if !line.contains("OsuScreenStack#") {
            return;
        }
        let name = line
            .split_once(" entered ")
            .or_else(|| line.split_once(" resume to "))
            .map(|(_, value)| value.split('#').next().unwrap_or(""));
        let Some(name) = name else {
            return;
        };
        let label = if name.contains("PlayerLoader") {
            "Загрузка карты"
        } else if name.contains("ReplayPlayer") {
            "Просмотр реплея"
        } else if name.contains("Player") {
            "Игровой экран"
        } else if name.contains("Results") {
            "Результаты"
        } else if name.contains("SongSelect") {
            "Выбор карты"
        } else if name.contains("MainMenu") {
            "Главное меню"
        } else {
            "Другой экран"
        };
        self.screen = Some(label.into());
    }
}

#[derive(Default)]
pub struct Tail {
    identity: (u32, u64),
    path: Option<PathBuf>,
    position: u64,
    pending: Vec<u8>,
    state: State,
    checked: Option<Instant>,
}

impl Tail {
    pub fn poll(
        &mut self,
        directory: &Path,
        pid: u32,
        started_ms: u64,
        now: Instant,
    ) -> Option<State> {
        if self.identity != (pid, started_ms) {
            *self = Self {
                identity: (pid, started_ms),
                ..Self::default()
            };
        }
        if pid == 0 || started_ms == 0 {
            return None;
        }
        if self
            .checked
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_millis(250))
        {
            return self.path.as_ref().map(|_| self.state.clone());
        }
        self.checked = Some(now);
        if self.path.is_none() {
            self.path = find_log(directory, started_ms);
        }
        let result = self.read_new();
        if result.is_err() {
            self.path = None;
            self.position = 0;
            self.pending.clear();
            self.state = State::default();
            return None;
        }
        self.path.as_ref().map(|_| self.state.clone())
    }

    fn read_new(&mut self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        if length < self.position {
            self.position = 0;
            self.pending.clear();
            self.state = State::default();
        }
        // Bound both the initial catch-up and later bursts. Never replay partial lines.
        if length.saturating_sub(self.position) > 2 * 1024 * 1024 {
            self.position = length - 2 * 1024 * 1024;
            self.pending.clear();
            self.state = State::default();
            file.seek(SeekFrom::Start(self.position))?;
            let mut first = [0; 1];
            while file.read(&mut first)? == 1 {
                self.position += 1;
                if first[0] == b'\n' {
                    break;
                }
            }
        }
        file.seek(SeekFrom::Start(self.position))?;
        let mut bytes = Vec::new();
        file.take(2 * 1024 * 1024).read_to_end(&mut bytes)?;
        self.position += bytes.len() as u64;
        self.pending.extend_from_slice(&bytes);
        let mut through = 0;
        for (index, byte) in self.pending.iter().enumerate() {
            if *byte == b'\n' {
                if index - through <= 16 * 1024 {
                    if let Ok(line) = std::str::from_utf8(&self.pending[through..index]) {
                        self.state.line(line);
                    }
                }
                through = index + 1;
            }
        }
        self.pending.drain(..through);
        if self.pending.len() > 16 * 1024 {
            return Err(std::io::Error::other("oversized runtime log line"));
        }
        Ok(())
    }
}

fn find_log(directory: &Path, started_ms: u64) -> Option<PathBuf> {
    let start = started_ms / 1000;
    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let seconds: u64 = name.to_str()?.strip_suffix(".runtime.log")?.parse().ok()?;
            (seconds >= start.saturating_sub(2) && seconds <= start.saturating_add(120))
                .then_some((seconds, entry.path()))
        })
        .min_by_key(|(seconds, _)| *seconds)
        .map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_committed_screen_transitions_and_current_map_messages_are_used() {
        let mut state = State::default();
        state.line("[verbose]: Game-wide working beatmap updated to Artist - Song (Mapper) [Hard]");
        state.line("[verbose]: 📺 OsuScreenStack#119(depth:4) entered SoloSongSelect#457");
        assert_eq!(state.screen.as_deref(), Some("Выбор карты"));
        state.line("[verbose]: 📺 OsuScreenStack#119(depth:4) suspended SoloSongSelect#457 (waiting on SoloPlayer#2)");
        assert_eq!(state.screen.as_deref(), Some("Выбор карты"));
        state.line("[verbose]: 📺 OsuScreenStack#119(depth:6) entered ReplayPlayer#2");
        assert_eq!(state.screen.as_deref(), Some("Просмотр реплея"));
        state.line("[verbose]: Invalidating working beatmap cache for Another map");
        assert_eq!(state.map.as_deref(), Some("Artist - Song (Mapper) [Hard]"));
    }
    #[test]
    fn tail_handles_partial_lines_truncation_and_process_restart_without_reusing_old_logs() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("dossier-lazer-tail-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1000.runtime.log");
        fs::write(&path, "Game-wide working beatmap updated to Map A\n").unwrap();
        let mut tail = Tail::default();
        let now = Instant::now();
        assert_eq!(
            tail.poll(&dir, 7, 1_000_000, now).unwrap().map.as_deref(),
            Some("Map A")
        );
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"Game-wide working beatmap updated to Map B")
            .unwrap();
        assert_eq!(
            tail.poll(&dir, 7, 1_000_000, now + Duration::from_secs(1))
                .unwrap()
                .map
                .as_deref(),
            Some("Map A")
        );
        file.write_all(b"\n").unwrap();
        assert_eq!(
            tail.poll(&dir, 7, 1_000_000, now + Duration::from_secs(2))
                .unwrap()
                .map
                .as_deref(),
            Some("Map B")
        );
        fs::write(&path, b"new log\n").unwrap();
        assert!(
            tail.poll(&dir, 7, 1_000_000, now + Duration::from_secs(3))
                .unwrap()
                .map
                .is_none()
        );
        assert!(
            tail.poll(&dir, 8, 2_000_000, now + Duration::from_secs(4))
                .is_none()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
