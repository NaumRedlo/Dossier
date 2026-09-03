//! Noticing that there is a newer Dossier, and becoming it.
//!
//! Two kinds of machine run this. One has the repository — the application was
//! built from it and `cargo` is right there — and for that machine an update is
//! `git pull` and a build, with every line of both put on the screen as it
//! happens. The other has only the application, and for it an update is a file
//! to download; all this can honestly do there is say that one exists and open
//! the page.
//!
//! Which of the two is decided by whether the source this was compiled from is
//! still on the disk, and that is the honest test: `CARGO_MANIFEST_DIR` is a
//! compile-time path, so in a bundle it names a directory on somebody else's
//! machine and simply is not there.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where the source is, if it is here at all.
pub fn checkout() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    root.join(".git").is_dir().then(|| root.to_path_buf())
}

#[derive(serde::Serialize)]
pub struct Update {
    pub version: String,
    /// `git` — buildable here; `release` — there is a newer one to download;
    /// `none` — nothing to do; `unknown` — could not find out.
    pub how: &'static str,
    /// How many commits behind, when that is knowable.
    pub behind: u32,
    pub newest: String,
    /// Which parts of the engine the update touches, by crate.
    pub parts: Vec<String>,
    pub said: String,
}

impl Update {
    fn nothing(said: &str) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            how: "none",
            behind: 0,
            newest: env!("CARGO_PKG_VERSION").to_owned(),
            parts: Vec::new(),
            said: said.to_owned(),
        }
    }
}

fn git(at: &Path, args: &[&str]) -> Result<String, String> {
    let done = Command::new("git")
        .current_dir(at)
        .args(args)
        .output()
        .map_err(|why| format!("git не запустился: {why}"))?;
    if !done.status.success() {
        return Err(String::from_utf8_lossy(&done.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&done.stdout).trim().to_owned())
}

/// Which crates a set of changed paths belongs to.
///
/// `crates/dossier-sim/src/judge.rs` is the engine's judging changing, and that
/// is worth saying; `crates/dossier-sim/src/lib.rs` is the same news twice.
fn parts_of(paths: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for path in paths.lines() {
        let name = match path.split('/').collect::<Vec<_>>()[..] {
            ["crates", crate_name, ..] => crate_name.to_owned(),
            ["app", ..] => "приложение".to_owned(),
            ["client", ..] => "клиент на Python".to_owned(),
            _ => continue,
        };
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen
}

/// Is there a newer one, and what would updating mean here.
pub fn look() -> Update {
    let Some(root) = checkout() else {
        return Update::nothing("обновление здесь не собирается — эта сборка пришла файлом");
    };
    if let Err(why) = git(&root, &["fetch", "--quiet", "origin"]) {
        return Update::nothing(&format!("не удалось спросить у origin: {why}"));
    }
    // `origin/HEAD` есть не в каждом клоне — его ставит только `git clone`, и
    // копия, сделанная иначе, о нём не знает. Тогда спрашиваем ветку по имени.
    let Some(ahead) = ["origin/HEAD", "origin/main"]
        .iter()
        .find(|name| git(&root, &["rev-parse", "--verify", "--quiet", name]).is_ok())
    else {
        return Update::nothing("не видно ветки origin — обновлять не от чего");
    };
    let behind: u32 = git(&root, &["rev-list", "--count", &format!("HEAD..{ahead}")])
        .ok()
        .and_then(|said| said.parse().ok())
        .unwrap_or(0);
    if behind == 0 {
        return Update::nothing("это самая свежая версия");
    }
    let changed = git(&root, &["diff", "--name-only", "HEAD", ahead]).unwrap_or_default();
    let dirty = git(&root, &["status", "--porcelain"]).unwrap_or_default();
    Update {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        how: if dirty.is_empty() { "git" } else { "dirty" },
        behind,
        newest: git(&root, &["rev-parse", "--short", ahead]).unwrap_or_default(),
        parts: parts_of(&changed),
        said: if dirty.is_empty() {
            String::new()
        } else {
            "в рабочей копии есть свои изменения — обновление их не тронет".to_owned()
        },
    }
}

/// Run one command and hand every line it says to `say`, as it says it.
///
/// stderr merged into stdout on purpose: `cargo` says everything worth showing
/// there — every `Compiling` line — and two streams read separately arrive in
/// the wrong order.
fn stream(
    at: &Path,
    program: &str,
    args: &[&str],
    say: &(dyn Fn(&str) + Sync),
) -> Result<(), String> {
    let mut child = Command::new(program)
        .current_dir(at)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("{program} не запустился: {why}"))?;

    let out = child.stdout.take().map(BufReader::new);
    let err = child.stderr.take().map(BufReader::new);
    std::thread::scope(|scope| {
        if let Some(out) = out {
            scope.spawn(|| {
                for line in out.lines().map_while(Result::ok) {
                    say(&line);
                }
            });
        }
        if let Some(err) = err {
            scope.spawn(|| {
                for line in err.lines().map_while(Result::ok) {
                    say(&line);
                }
            });
        }
    });

    let done = child.wait().map_err(|why| why.to_string())?;
    if done.success() {
        Ok(())
    } else {
        Err(format!("{program} закончил с ошибкой"))
    }
}

/// Pull and build, saying everything as it happens.
pub fn run(say: &(dyn Fn(&str) + Sync)) -> Result<(), String> {
    let root = checkout().ok_or("исходников нет — обновлять нечего")?;
    say("git pull --ff-only");
    stream(&root, "git", &["pull", "--ff-only"], say)?;
    say("cargo build --release");
    stream(&root.join("app"), "cargo", &["build", "--release"], say)?;
    Ok(())
}

/// A build log with somebody's home directory taken out of it.
///
/// Every path in a `cargo` error starts with it, and a report is about the
/// build and not about whose machine it happened on.
pub fn tidy(log: &str) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() {
        return log.to_owned();
    }
    log.replace(&home, "~")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_file_names_its_crate_once() {
        let said = parts_of(
            "crates/dossier-sim/src/judge.rs\ncrates/dossier-sim/src/lib.rs\napp/ui/app.js\nREADME.md\n",
        );
        assert_eq!(said, vec!["dossier-sim", "приложение"]);
    }

    #[test]
    fn a_home_directory_does_not_travel_with_a_report() {
        std::env::set_var("HOME", "/Users/somebody");
        let said = tidy("error at /Users/somebody/Documents/Dossier/app/src/main.rs:1");
        assert_eq!(said, "error at ~/Documents/Dossier/app/src/main.rs:1");
    }

    #[test]
    fn nothing_to_do_still_names_this_version() {
        let said = Update::nothing("тест");
        assert_eq!(said.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(said.how, "none");
        assert!(said.parts.is_empty());
    }
}
