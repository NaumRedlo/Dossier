use md5::{Digest, Md5};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dossier-corpus-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let map = "osu file format v14\n[General]\nMode:0\n[Metadata]\nTitle:Corpus fixture\nArtist:Test\nVersion:Test\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:5\nApproachRate:5\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,1,100,1,0\n[HitObjects]\n256,192,1000,1,0,0:0:0:0:\n";
        fs::write(dir.join("map.osu"), map).unwrap();
        let mut replay = include_bytes!("../../../native/tests/fixtures/witness.osr").to_vec();
        let old_hash = dossier_replay::Replay::parse(&replay).unwrap().beatmap_hash;
        let start = replay
            .windows(old_hash.len())
            .position(|s| s == old_hash.as_bytes())
            .unwrap();
        let hash = format!("{:x}", Md5::digest(map.as_bytes()));
        replay[start..start + 32].copy_from_slice(hash.as_bytes());
        fs::write(dir.join("valid.osr"), replay).unwrap();
        fs::write(dir.join("broken.osr"), b"truncated").unwrap();
        Self(dir)
    }
    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_dossier"))
            .current_dir(&self.0)
            .args(["corpus", "--songs", ".", "--threads", "1"])
            .args(arguments)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_failed_replay_cannot_hide_behind_a_successful_one_or_a_count_ceiling() {
    let fixture = Fixture::new();
    let good = fixture.run(&["--strict", "100000", "valid.osr"]);
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stderr)
    );
    let mixed = fixture.run(&["--strict", "100000", "valid.osr", "broken.osr"]);
    assert!(!mixed.status.success());
    assert!(String::from_utf8_lossy(&mixed.stderr).contains("broken.osr"));
    assert!(String::from_utf8_lossy(&mixed.stderr).contains("incomplete"));
}

#[test]
fn an_incomplete_update_does_not_replace_the_baseline() {
    let fixture = Fixture::new();
    let saved = fixture.run(&["--expect", "baseline.tsv", "--update-expect", "valid.osr"]);
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let before = fs::read(fixture.0.join("baseline.tsv")).unwrap();
    let failed = fixture.run(&[
        "--expect",
        "baseline.tsv",
        "--update-expect",
        "--prune",
        "valid.osr",
        "broken.osr",
    ]);
    assert!(!failed.status.success());
    assert_eq!(fs::read(fixture.0.join("baseline.tsv")).unwrap(), before);
}

#[test]
fn no_measurements_is_a_failure_even_without_strict_mode() {
    let fixture = Fixture::new();
    for file in ["broken.osr", "missing.osr"] {
        let result = fixture.run(&[file]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains(file));
    }
}

#[test]
fn a_numeric_ceiling_also_requires_the_expected_replays_to_be_present() {
    let fixture = Fixture::new();
    assert!(fixture
        .run(&["--expect", "baseline.tsv", "--update-expect", "valid.osr"])
        .status
        .success());
    let mut baseline = fs::read_to_string(fixture.0.join("baseline.tsv")).unwrap();
    baseline.push_str(&format!(
        "{}\t{}\t-\t0\t0\t-\telsewhere.osr\n",
        "f".repeat(32),
        "e".repeat(32)
    ));
    fs::write(fixture.0.join("baseline.tsv"), baseline).unwrap();
    let result = fixture.run(&[
        "--expect",
        "baseline.tsv",
        "--strict",
        "100000",
        "valid.osr",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("not on this machine"));
}
