use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const NAME: &str = "witness.alive";
pub const SLACK: Duration = Duration::from_secs(20);
pub const EVERY: Duration = Duration::from_secs(5);

pub fn beside(program: &Path) -> Option<PathBuf> {
    program.parent().map(|dir| dir.join(NAME))
}

pub fn held(leash: &Path, now: SystemTime) -> bool {
    match std::fs::metadata(leash).and_then(|meta| meta.modified()) {
        Ok(touched) => now.duration_since(touched).map_or(true, |age| age < SLACK),
        Err(_) => false,
    }
}

pub fn touch(leash: &Path) -> bool {
    std::fs::write(leash, b"held").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leash_holds_while_it_is_touched_and_lets_go_when_it_is_not() {
        let dir = std::env::temp_dir().join(format!("witness-leash-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let leash = beside(&dir.join("witness.exe")).expect("a place beside the program");
        assert_eq!(leash, dir.join(NAME));
        assert!(!held(&leash, SystemTime::now()), "a leash nobody holds was taken for a held one");
        assert!(touch(&leash));
        let now = SystemTime::now();
        assert!(held(&leash, now));
        assert!(held(&leash, now + SLACK - Duration::from_secs(2)));
        assert!(!held(&leash, now + SLACK + Duration::from_secs(2)));
        assert!(held(&leash, now - Duration::from_secs(60)), "a clock that stepped back let go of a live leash");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
