use std::io::Write;
use std::path::Path;

use crate::json::quote;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Events(bool);

impl Events {
    pub fn wanted(asked: bool) -> Self {
        Self(asked)
    }

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

    pub fn clip(self, index: usize, of: usize, at_ms: f64, reason: &str) {
        self.say(&format!(
            "{{\"event\":\"clip\",\"index\":{index},\"of\":{of},\"at_ms\":{at_ms:.1},\
             \"reason\":{}}}",
            quote(reason)
        ));
    }

    pub fn progress(self, frames: u64, of: u64, per_second: f64, left_seconds: f64) {
        self.say(&format!(
            "{{\"event\":\"progress\",\"frames\":{frames},\"of\":{of},\
             \"per_second\":{per_second:.1},\"left_seconds\":{left_seconds:.1}}}"
        ));
    }

    pub fn video(self, width: u32, height: u32, seconds: f64) {
        self.say(&format!(
            "{{\"event\":\"video\",\"width\":{width},\"height\":{height},\
             \"seconds\":{seconds:.3}}}"
        ));
    }

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
        assert_eq!(Events::wanted(false), Events(false));
    }

    #[test]
    fn a_reason_with_quotes_in_it_stays_one_line_of_json() {
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

pub fn listen(to: impl Fn(&str) + Send + Sync + 'static) {
    *sink().lock().expect("the event sink was poisoned") = Some(Box::new(to));
}

pub fn unlisten() {
    *sink().lock().expect("the event sink was poisoned") = None;
}

#[cfg(test)]
mod listening {
    use std::sync::{Arc, Mutex};

    #[test]
    fn a_listener_hears_the_render_happening() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mine = Arc::clone(&heard);
        super::listen(move |line| mine.lock().expect("heard").push(line.to_owned()));
        super::Events::wanted(true).progress(12, 48, 300.0, 0.12);

        super::Events::wanted(false).progress(1, 2, 1.0, 1.0);
        super::unlisten();
        let said = heard.lock().expect("heard").clone();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("\"event\":\"progress\""), "{}", said[0]);
        assert!(said[0].contains("\"frames\":12"), "{}", said[0]);
        assert!(said[0].contains("\"of\":48"), "{}", said[0]);
    }
}
