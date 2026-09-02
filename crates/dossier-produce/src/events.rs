//! What a long render is doing, in a form a program can read.
//!
//! Everything this engine says about a render it says to stderr, in sentences
//! meant for a person watching one happen. The bot needs three of those facts
//! while they are still true — how far along the frames are, which clip of a
//! reel they belong to, and the shape of the finished file, which Telegram
//! needs in advance or it draws a square placeholder for a widescreen video.
//!
//! It was getting them by matching regular expressions against that prose. The
//! two halves were each defensible on their own and the seam between them was
//! not: rewording a progress line — a person's sentence, in a file about
//! drawing — silently stopped a live counter in a Telegram chat, and no test
//! on either side could notice, because neither side was wrong.
//!
//! So `--events` opens a second channel. Facts go to stdout as one JSON object
//! per line, the prose stays on stderr exactly as it was, and each is free to
//! change without the other. Nothing is emitted at all unless it is asked for,
//! which keeps a person's terminal a person's terminal.

use std::io::Write;
use std::path::Path;

use crate::json::quote;

/// Whether this run is being watched by a program, and the one place that
/// decides what such a watcher is told.
///
/// Carried rather than read from a global, so that the answer to "does this
/// render report itself" arrives by the same road as every other setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Events(bool);

impl Events {
    /// On when the caller asked for it, and silent otherwise.
    pub fn wanted(asked: bool) -> Self {
        Self(asked)
    }

    /// One object, one line, flushed. A watcher reads these while the render
    /// is still running, so a line held in a buffer is a line that arrives too
    /// late to be worth anything.
    ///
    /// Unless somebody is [`listen`]ing, in which case it goes to them and
    /// nothing is written: an application watching its own render is not going
    /// to read its own stdout, and the bot wants these while the frames are
    /// still being drawn.
    fn say(self, line: &str) {
        if !self.0 {
            return;
        }
        if let Some(to) = sink().lock().expect("the event sink was poisoned").as_ref() {
            to(line);
            return;
        }
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }

    /// A reel is about to draw its next clip.
    ///
    /// Frames count from zero once per clip, so a counter built from frames
    /// alone reaches a hundred per cent once per clip too — which reads as a
    /// render restarting. This is what tells the two apart.
    pub fn clip(self, index: usize, of: usize, at_ms: f64, reason: &str) {
        self.say(&format!(
            "{{\"event\":\"clip\",\"index\":{index},\"of\":{of},\"at_ms\":{at_ms:.1},\
             \"reason\":{}}}",
            quote(reason)
        ));
    }

    /// How far along the frames are.
    pub fn progress(self, frames: u64, of: u64, per_second: f64, left_seconds: f64) {
        self.say(&format!(
            "{{\"event\":\"progress\",\"frames\":{frames},\"of\":{of},\
             \"per_second\":{per_second:.1},\"left_seconds\":{left_seconds:.1}}}"
        ));
    }

    /// The shape of a finished file, from the process that wrote it.
    ///
    /// A reel says this once per clip and once more for the file it cut them
    /// into; the last one is the one that describes what was actually made.
    pub fn video(self, width: u32, height: u32, seconds: f64) {
        self.say(&format!(
            "{{\"event\":\"video\",\"width\":{width},\"height\":{height},\
             \"seconds\":{seconds:.3}}}"
        ));
    }

    /// The file is on disk and this is where.
    pub fn wrote(self, path: &Path, bytes: u64) {
        self.say(&format!(
            "{{\"event\":\"wrote\",\"path\":{},\"bytes\":{bytes}}}",
            quote(&path.display().to_string())
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_the_default_shape_of_this() {
        // Not a formatting test — the point is that a run nobody asked to
        // watch writes nothing at all to stdout, where `exhibit` prints the
        // path of the file it made.
        assert_eq!(Events::wanted(false), Events(false));
    }

    #[test]
    fn a_reason_with_quotes_in_it_stays_one_line_of_json() {
        // Reasons are prose, written elsewhere, and a map called `"osu!"` is
        // not a reason to hand a watcher a broken stream.
        let reason = "a \"1425x\" run\nbreaks";
        let line = format!("{{\"reason\":{}}}", quote(reason));
        assert_eq!(line, r#"{"reason":"a \"1425x\" run\nbreaks"}"#);
        assert_eq!(line.lines().count(), 1);
    }
}

type Sink = Box<dyn Fn(&str) + Send + Sync>;

fn sink() -> &'static std::sync::Mutex<Option<Sink>> {
    static SINK: std::sync::OnceLock<std::sync::Mutex<Option<Sink>>> = std::sync::OnceLock::new();
    SINK.get_or_init(|| std::sync::Mutex::new(None))
}

/// Read the render's events in this process instead of off a pipe.
///
/// The bot's worker used to run the engine as a program and parse its stdout.
/// An application that *is* the engine has no pipe to read, and the same lines
/// are what it wants to put in front of somebody: how far along, how fast, how
/// long is left.
pub fn listen(to: impl Fn(&str) + Send + Sync + 'static) {
    *sink().lock().expect("the event sink was poisoned") = Some(Box::new(to));
}

/// Stop listening; events go back to stdout.
pub fn unlisten() {
    *sink().lock().expect("the event sink was poisoned") = None;
}

#[cfg(test)]
mod listening {
    use std::sync::{Arc, Mutex};

    /// The line a watcher would have read off the pipe, delivered instead.
    #[test]
    fn a_listener_hears_the_render_happening() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mine = Arc::clone(&heard);
        super::listen(move |line| mine.lock().expect("heard").push(line.to_owned()));
        super::Events::wanted(true).progress(12, 48, 300.0, 0.12);
        // Off, and it says nothing to anybody — the same silence as before.
        super::Events::wanted(false).progress(1, 2, 1.0, 1.0);
        super::unlisten();
        let said = heard.lock().expect("heard").clone();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("\"event\":\"progress\""), "{}", said[0]);
        assert!(said[0].contains("\"frames\":12"), "{}", said[0]);
        assert!(said[0].contains("\"of\":48"), "{}", said[0]);
    }
}
