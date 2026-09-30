use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const SLOW: Duration = Duration::from_millis(4);
const APART: Duration = Duration::from_millis(250);
const NAME: usize = 40;

struct Log {
    out: std::io::BufWriter<std::fs::File>,
    began: Instant,
    last: Option<Instant>,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);
static ON: AtomicBool = AtomicBool::new(false);

pub fn start() {
    let Some(path) = std::env::var_os("DOSSIER_FRAMES") else {
        return;
    };
    if let Ok(file) = std::fs::File::create(path) {
        *LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Log { out: std::io::BufWriter::new(file), began: Instant::now(), last: None });
        ON.store(true, Ordering::Relaxed);
    }
}

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

struct Short(String);

impl std::fmt::Write for Short {
    fn write_str(&mut self, said: &str) -> std::fmt::Result {
        for c in said.chars() {
            if matches!(c, '(' | ' ' | '{') || self.0.len() >= NAME {
                return Err(std::fmt::Error);
            }
            self.0.push(c);
        }
        Ok(())
    }
}

pub fn name(of: &impl std::fmt::Debug) -> String {
    let mut short = Short(String::new());
    let _ = write!(short, "{of:?}");
    short.0
}

fn write(kind: &str, at: Instant, took: Duration, label: &str) {
    let mut log = LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(log) = log.as_mut() else {
        return;
    };
    let since = at.saturating_duration_since(log.began).as_secs_f64() * 1000.0;
    let _ = writeln!(log.out, "{kind}\t{since:.1}\t{:.2}\t{label}", took.as_secs_f64() * 1000.0);
    let _ = log.out.flush();
}

pub fn frame(at: Instant, label: &str) {
    if !on() {
        return;
    }
    let last = {
        let mut log = LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(log) = log.as_mut() else {
            return;
        };
        log.last.replace(at)
    };
    if let Some(gap) = last.map(|last| at.saturating_duration_since(last)).filter(|gap| *gap < APART) {
        write("frame", at, gap, label);
    }
}

pub fn slow(kind: &str, from: Instant, label: &str) {
    let took = from.elapsed();
    if on() && took >= SLOW {
        write(kind, from, took, label);
    }
}

#[cfg(test)]
mod tests {
    use super::name;

    #[derive(Debug)]
    #[allow(dead_code)]
    enum Said {
        Tick(u32),
        Loaded { entries: Vec<u32> },
        Nudged,
    }

    #[test]
    fn a_message_is_named_by_its_variant_without_its_load() {
        assert_eq!(name(&Said::Tick(7)), "Tick");
        assert_eq!(name(&Said::Loaded { entries: vec![1; 100_000] }), "Loaded");
        assert_eq!(name(&Said::Nudged), "Nudged");
    }
}
