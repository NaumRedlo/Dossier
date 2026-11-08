use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::sources::{Kind, Source};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub files: Vec<PathBuf>,
    pub bytes: u64,
    pub local: usize,
    pub stored: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Done {
    pub removed: Vec<PathBuf>,
    pub stayed: Vec<(PathBuf, String)>,
}

pub fn in_store(path: &Path, sources: &[Source]) -> bool {
    sources.iter().any(|source| source.kind == Kind::Lazer && path.starts_with(source.root.join("files")))
}

pub fn local_score(path: &Path, sources: &[Source]) -> bool {
    sources.iter().any(|source| source.kind == Kind::Stable && path.starts_with(source.root.join("Data").join("r")))
}

pub fn plan(paths: impl IntoIterator<Item = PathBuf>, sources: &[Source]) -> Plan {
    let mut plan = Plan::default();
    let mut seen = BTreeSet::new();
    for path in paths {
        if !seen.insert(path.clone()) {
            continue;
        }
        if in_store(&path, sources) {
            plan.stored += 1;
            continue;
        }
        if local_score(&path, sources) {
            plan.local += 1;
        }
        plan.bytes += std::fs::metadata(&path).map_or(0, |meta| meta.len());
        plan.files.push(path);
    }
    plan
}

pub fn carry_out(files: &[PathBuf], bin: impl Fn(&Path) -> Result<(), String>) -> Done {
    let mut done = Done::default();
    for path in files {
        match bin(path) {
            Ok(()) => done.removed.push(path.clone()),
            Err(_) if !path.exists() => done.removed.push(path.clone()),
            Err(why) => done.stayed.push((path.clone(), why)),
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(kind: Kind, root: &Path) -> Source {
        Source { kind, root: root.to_path_buf(), songs: None, skins: None, replays: None, maps: None, skin_count: 0, replay_count: 0, scores: 0, on: true }
    }

    #[test]
    fn lazer_storage_is_never_planned_and_stable_local_scores_are_counted() {
        let dir = tempfile::tempdir().unwrap();
        let (stable, lazer, own) = (dir.path().join("osu!"), dir.path().join("lazer"), dir.path().join("mine"));
        for folder in [stable.join("Data").join("r"), stable.join("Replays"), lazer.join("files").join("ab"), lazer.join("exports"), own.clone()] {
            std::fs::create_dir_all(folder).unwrap();
        }
        let local = stable.join("Data").join("r").join("a-1.osr");
        let exported = stable.join("Replays").join("b.osr");
        let stored = lazer.join("files").join("ab").join("abcdef");
        let lazer_export = lazer.join("exports").join("c.osr");
        let added = own.join("d.osr");
        for (path, size) in [(&local, 10), (&exported, 20), (&stored, 40), (&lazer_export, 80), (&added, 160)] {
            std::fs::write(path, vec![0u8; size]).unwrap();
        }
        let sources = [source(Kind::Stable, &stable), source(Kind::Lazer, &lazer), source(Kind::Folder, &own)];
        let plan = plan([local.clone(), exported.clone(), stored, lazer_export.clone(), added.clone(), exported.clone()], &sources);
        assert_eq!(plan.files, vec![local, exported, lazer_export, added]);
        assert_eq!((plan.bytes, plan.local, plan.stored), (270, 1, 1));
    }

    #[test]
    fn a_file_that_could_not_be_moved_stays_with_its_reason_and_a_vanished_one_counts_as_gone() {
        let dir = tempfile::tempdir().unwrap();
        let held = dir.path().join("held.osr");
        let fine = dir.path().join("fine.osr");
        std::fs::write(&held, b"x").unwrap();
        std::fs::write(&fine, b"x").unwrap();
        let gone = dir.path().join("gone.osr");
        let done = carry_out(&[fine.clone(), held.clone(), gone.clone()], |path| {
            if path == held || path == gone { Err("in use".to_owned()) } else { std::fs::remove_file(path).map_err(|e| e.to_string()) }
        });
        assert_eq!(done.removed, vec![fine, gone]);
        assert_eq!(done.stayed, vec![(held, "in use".to_owned())]);
    }
}
