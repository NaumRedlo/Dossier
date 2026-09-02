//! Where the pipeline says the things it used to print.
//!
//! It said them on stderr, which is right for a command line and invisible in
//! a window. "No font found — drawing without numbers" is exactly the sort of
//! thing somebody needs to see, and exactly the sort an application throws
//! away: nobody is watching the stream a windowed process was started with.
//!
//! So they are *handed out* rather than printed, and whoever is listening
//! decides. Nothing changes for the command line, which is what the default
//! does — the same line, on the same stream, with the same prefix.
//!
//! Warnings only. The render's running commentary — how many threads are
//! drawing, which clip is being cut, the progress line that rewrites itself —
//! stays on stderr, because it is a terminal's display and an application
//! wants [`crate::events`] instead, which says the same things in a form a
//! program can read.
//!
//! One sink for the process rather than a channel threaded through every
//! signature. Some of these are said from inside the frame loop, which runs on
//! several threads, and a `&mut` cannot go there; and there is one pipeline in
//! a process, so there is one place to say things to.

use std::sync::{Mutex, OnceLock};

type Sink = Box<dyn Fn(&str) + Send + Sync>;

fn sink() -> &'static Mutex<Option<Sink>> {
    static SINK: OnceLock<Mutex<Option<Sink>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

/// Listen to what the pipeline has to say, instead of letting it print.
///
/// The text arrives without a prefix and without a newline: a window puts it in
/// a list, and a terminal wants `dossier: ` in front of it, and neither should
/// have to take the other's decoration off.
pub fn listen(to: impl Fn(&str) + Send + Sync + 'static) {
    *sink().lock().expect("the note sink was poisoned") = Some(Box::new(to));
}

/// Stop listening; notes go back to stderr.
pub fn unlisten() {
    *sink().lock().expect("the note sink was poisoned") = None;
}

/// Say something that does not stop the render.
///
/// Called through [`note!`](crate::note), which is how it reads at the places
/// that use it.
pub fn say(text: &str) {
    let held = sink().lock().expect("the note sink was poisoned");
    match held.as_ref() {
        Some(to) => to(text),
        None => eprintln!("dossier: {text}"),
    }
}

/// `note!("could not read {name}")` — the pipeline's own `eprintln!`.
#[macro_export]
macro_rules! note {
    ($($arg:tt)*) => {
        $crate::notes::say(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    #[test]
    fn a_listener_hears_what_would_have_been_printed() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mine = Arc::clone(&heard);
        super::listen(move |text| mine.lock().expect("heard").push(text.to_owned()));
        note!("the font is {}", "missing");
        super::unlisten();
        // And nothing is added on the way: the prefix belongs to whoever is
        // showing it, not to whoever said it.
        assert_eq!(*heard.lock().expect("heard"), vec!["the font is missing"]);
    }
}
