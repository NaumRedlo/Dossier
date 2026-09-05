use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const INPUTS: [&str; 3] = ["crates", "Cargo.lock", "Cargo.toml"];

fn main() {
    let root = workspace_root();

    for input in INPUTS {
        println!("cargo:rerun-if-changed={}", root.join(input).display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join(".git/HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("../.git/HEAD").display()
    );

    let id = describe(&root).unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=DOSSIER_BUILD={id}");
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    manifest
        .parent()
        .and_then(Path::parent)
        .map_or(manifest.clone(), Path::to_path_buf)
}

fn describe(root: &Path) -> Option<String> {
    let mut args = vec!["rev-parse".to_owned()];
    args.extend(INPUTS.iter().map(|input| format!("HEAD:./{input}")));
    let ids = git(root, &args, None)?;

    let folded = git(
        root,
        &["hash-object".to_owned(), "--stdin".to_owned()],
        Some(&ids),
    )?;
    let short: String = folded.chars().take(7).collect();
    if short.len() < 7 {
        return None;
    }

    let mut status = vec![
        "status".to_owned(),
        "--porcelain".to_owned(),
        "--untracked-files=no".to_owned(),
        "--".to_owned(),
    ];
    status.extend(INPUTS.iter().map(|input| (*input).to_owned()));
    let dirty = git(root, &status, None).is_some_and(|out| !out.is_empty());

    Some(if dirty { format!("{short}+") } else { short })
}

fn git(root: &Path, args: &[String], stdin: Option<&str>) -> Option<String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(text) = stdin {
        child.stdin.take()?.write_all(text.as_bytes()).ok()?;
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8(out.stdout).ok()?.trim().to_owned())
}
