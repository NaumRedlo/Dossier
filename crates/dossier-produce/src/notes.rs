use std::sync::{Mutex, OnceLock};

type Sink = Box<dyn Fn(&str) + Send + Sync>;

fn sink() -> &'static Mutex<Option<Sink>> {
    static SINK: OnceLock<Mutex<Option<Sink>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

pub fn listen(to: impl Fn(&str) + Send + Sync + 'static) {
    *sink().lock().expect("the note sink was poisoned") = Some(Box::new(to));
}

pub fn unlisten() {
    *sink().lock().expect("the note sink was poisoned") = None;
}

pub fn say(text: &str) {
    let held = sink().lock().expect("the note sink was poisoned");
    match held.as_ref() {
        Some(to) => to(text),
        None => eprintln!("dossier: {text}"),
    }
}

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

        assert_eq!(*heard.lock().expect("heard"), vec!["the font is missing"]);
    }
}
