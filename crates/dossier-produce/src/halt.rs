use std::sync::atomic::{AtomicBool, Ordering};

static ASKED: AtomicBool = AtomicBool::new(false);

pub const SAID: &str = "остановлено";

pub fn ask() {
    ASKED.store(true, Ordering::Relaxed);
}

pub fn asked() -> bool {
    ASKED.load(Ordering::Relaxed)
}

pub fn clear() {
    ASKED.store(false, Ordering::Relaxed);
}

pub fn was_it(message: &str) -> bool {
    message.starts_with(SAID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_halt_stays_asked_until_it_is_cleared() {
        clear();
        assert!(!asked());
        ask();
        assert!(asked());
        assert!(asked());
        clear();
        assert!(!asked());
    }

    #[test]
    fn a_halt_is_told_apart_from_a_failure_by_its_first_word() {
        assert!(was_it(SAID));
        assert!(was_it(&format!("{SAID} на 412 кадре")));
        assert!(!was_it("ffmpeg stopped after 412 frames"));
    }
}
