use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

const RETRY: Duration = Duration::from_secs(30);

/// Owns a worker so Windows acquires and releases its thread-local request on
/// the same thread, and Linux session-bus calls never block the UI.
pub(crate) struct PlaybackWake {
    sender: Sender<bool>,
    playing: bool,
}

impl PlaybackWake {
    pub(crate) fn new() -> Self { Self::with_factory(acquire) }

    fn with_factory<G: 'static>(make: impl FnMut() -> Result<G, String> + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new().name("dossier-playback-power".into())
            .spawn(move || run(receiver, make)).expect("playback power worker");
        Self { sender, playing: false }
    }

    pub(crate) fn playing(&mut self, playing: bool) {
        if self.playing != playing {
            self.playing = playing;
            let _ = self.sender.send(playing);
        }
    }
}

fn run<G>(receiver: Receiver<bool>, mut make: impl FnMut() -> Result<G, String>) {
    let mut wanted = false;
    let mut guard = None;
    loop {
        let next = if wanted && guard.is_none() { receiver.recv_timeout(RETRY) }
            else { receiver.recv().map_err(|_| RecvTimeoutError::Disconnected) };
        match next {
            Ok(playing) => wanted = playing,
            Err(RecvTimeoutError::Timeout) => {},
            Err(RecvTimeoutError::Disconnected) => break,
        }
        while let Ok(playing) = receiver.try_recv() { wanted = playing; }
        if wanted && guard.is_none() {
            match make() {
                Ok(awake) => guard = Some(awake),
                Err(why) => eprintln!("Dossier playback power request: {why}"),
            }
        } else if !wanted { guard = None; }
    }
    // Release on this worker, including when the player or application is dropped.
    drop(guard);
}

struct SystemWake(Vec<keepawake::KeepAwake>);

impl Drop for SystemWake {
    fn drop(&mut self) {
        // A disappearing Linux session bus must not unwind the player worker.
        while let Some(request) = self.0.pop() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(request)));
        }
    }
}

fn request(display: bool, idle: bool) -> Result<keepawake::KeepAwake, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| keepawake::Builder::default()
        .display(display).idle(idle).reason("Dossier video playback")
        .app_name("Dossier").app_reverse_domain("io.dossier.app").create()))
        .map_err(|_| "power service disconnected".to_owned())?
        .map_err(|e| e.to_string())
}

fn acquire() -> Result<SystemWake, String> {
    #[cfg(not(target_os = "linux"))]
    { request(true, true).map(|request| SystemWake(vec![request])) }
    #[cfg(target_os = "linux")]
    {
        // The session screensaver and logind are independent services. Keep a
        // supported request even if the other service is unavailable.
        let mut requests = Vec::new();
        let mut errors = Vec::new();
        for (display, idle) in [(true, false), (false, true)] {
            match request(display, idle) {
                Ok(request) => requests.push(request),
                Err(why) => errors.push(why),
            }
        }
        if requests.is_empty() { Err(errors.join("; ")) } else { Ok(SystemWake(requests)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Guard(Sender<&'static str>);
    impl Drop for Guard { fn drop(&mut self) { let _ = self.0.send("released"); } }

    #[test]
    fn pause_resume_and_drop_release_each_request_on_its_worker() {
        let (events, seen) = mpsc::channel();
        let threads = Arc::new(Mutex::new(Vec::new()));
        let recorded = threads.clone();
        let mut wake = PlaybackWake::with_factory(move || {
            recorded.lock().unwrap().push(std::thread::current().id());
            events.send("acquired").unwrap();
            Ok(Guard(events.clone()))
        });
        assert!(seen.try_recv().is_err());
        wake.playing(true);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "acquired");
        wake.playing(true);
        assert!(seen.try_recv().is_err(), "repeated playing updates do not acquire twice");
        wake.playing(false);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "released");
        wake.playing(true);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "acquired");
        drop(wake);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "released");
        let ids = threads.lock().unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[0], std::thread::current().id());
    }

    #[test]
    fn a_failed_request_does_not_block_pause_or_a_later_attempt() {
        let (events, seen) = mpsc::channel();
        let mut attempt = 0;
        let mut wake = PlaybackWake::with_factory(move || {
            attempt += 1;
            if attempt == 1 { events.send("failed").unwrap(); Err("no service".into()) }
            else { events.send("acquired").unwrap(); Ok(Guard(events.clone())) }
        });
        wake.playing(true);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "failed");
        wake.playing(false);
        wake.playing(true);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "acquired");
        wake.playing(false);
        assert_eq!(seen.recv_timeout(Duration::from_secs(2)).unwrap(), "released");
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "checks live macOS power assertions"]
    fn macos_power_assertions_are_visible_and_released() {
        let assertions = || String::from_utf8(std::process::Command::new("pmset").args(["-g", "assertions"]).output().unwrap().stdout).unwrap();
        assert!(!assertions().contains("Dossier video playback"));
        let awake = acquire().expect("macOS power request");
        let shown = assertions();
        assert!(shown.contains("Dossier video playback"));
        assert!(shown.contains("PreventUserIdleDisplaySleep") && shown.contains("PreventUserIdleSystemSleep"));
        drop(awake);
        assert!(!assertions().contains("Dossier video playback"));
    }
}
