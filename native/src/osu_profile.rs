use serde_json::Value;

use crate::community::wire::{Card, Grades, Score};

const SITE: &str = "https://osu.ppy.sh";

fn number(value: &Value) -> f64 {
    value.as_f64().or_else(|| value.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

fn words(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

pub fn card_from_page(page: &str) -> Option<Card> {
    let raw = page.split("data-initial-data=\"").nth(1)?.split('"').next()?;
    let data: Value = serde_json::from_str(&crate::news::unescaped(raw)).ok()?;
    let user = data.get("user")?;
    let stats = &user["statistics"];
    let grades = &stats["grade_counts"];
    Some(Card {
        username: words(&user["username"]),
        osu_id: number(&user["id"]),
        pp: number(&stats["pp"]),
        global_rank: number(&stats["global_rank"]),
        country: words(&user["country_code"]),
        country_name: words(&user["country"]["name"]),
        country_rank: number(&stats["country_rank"]),
        accuracy: number(&stats["hit_accuracy"]),
        play_count: number(&stats["play_count"]),
        play_seconds: number(&stats["play_time"]),
        ranked_score: number(&stats["ranked_score"]),
        total_hits: number(&stats["total_hits"]),
        total_score: number(&stats["total_score"]),
        level: number(&stats["level"]["current"]),
        level_progress: number(&stats["level"]["progress"]),
        maximum_combo: number(&stats["maximum_combo"]),
        replays_watched: number(&stats["replays_watched_by_others"]),
        grade_counts: Grades { a: number(&grades["a"]), s: number(&grades["s"]), sh: number(&grades["sh"]), ss: number(&grades["ss"]), ssh: number(&grades["ssh"]) },
        is_online: user["is_online"].as_bool().unwrap_or(false),
        is_supporter: user["is_supporter"].as_bool().unwrap_or(false),
        join_date: words(&user["join_date"]),
        last_visit: words(&user["last_visit"]),
        avatar_url: words(&user["avatar_url"]),
        cover_url: user["cover"]["url"].as_str().or_else(|| user["cover_url"].as_str()).unwrap_or_default().to_owned(),
        rank_history: user["rank_history"]["data"].as_array().map(|days| days.iter().map(number).filter(|rank| *rank > 0.0).collect()).unwrap_or_default(),
        ..Card::default()
    })
}

pub fn scores_from(json: &str) -> Vec<Score> {
    let Ok(Value::Array(list)) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    list.iter()
        .map(|score| {
            let statistics = &score["statistics"];
            let mods: Vec<String> = score["mods"]
                .as_array()
                .map(|mods| mods.iter().filter_map(|m| m["acronym"].as_str().or_else(|| m.as_str())).filter(|m| *m != "CL").map(str::to_owned).collect())
                .unwrap_or_default();
            Score {
                rank: words(&score["rank"]),
                artist: words(&score["beatmapset"]["artist"]),
                title: words(&score["beatmapset"]["title"]),
                version: words(&score["beatmap"]["version"]),
                pp: number(&score["pp"]),
                accuracy: number(&score["accuracy"]) * 100.0,
                max_combo: number(&score["max_combo"]),
                mods: mods.join(","),
                beatmap_id: number(&score["beatmap_id"]),
                beatmapset_id: number(&score["beatmapset"]["id"]),
                creator: words(&score["beatmapset"]["creator"]),
                great: number(&statistics["great"]),
                ok: number(&statistics["ok"]),
                meh: number(&statistics["meh"]),
                miss: number(&statistics["miss"]),
                stars: number(&score["beatmap"]["difficulty_rating"]),
                map_max_combo: number(&score["beatmap"]["max_combo"]),
                bpm: number(&score["beatmap"]["bpm"]),
                length: number(&score["beatmap"]["total_length"]),
                played: score["ended_at"].as_str().or_else(|| score["created_at"].as_str()).unwrap_or_default().to_owned(),
                ..Score::default()
            }
        })
        .collect()
}

pub fn fetch(name: &str) -> Result<Card, String> {
    let name = name.trim().trim_start_matches('@');
    if name.is_empty() {
        return Err("no name".to_owned());
    }
    let page = crate::news::page(&format!("{SITE}/users/{}", name.replace(' ', "%20")))?;
    let mut card = card_from_page(&page).ok_or("the profile page carries no profile")?;
    if card.osu_id > 0.0 {
        if let Ok(json) = crate::news::page(&format!("{SITE}/users/{}/scores/best?mode=osu&limit=5", card.osu_id as u64)) {
            card.top_scores = scores_from(&json);
        }
    }
    Ok(card)
}

pub fn cache_path() -> std::path::PathBuf {
    crate::sources::own_root().join("osu-profile.json")
}

pub fn load() -> Option<Card> {
    std::fs::read(cache_path()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn save(card: &Card) {
    if let Ok(bytes) = serde_json::to_vec(card) {
        let _ = std::fs::create_dir_all(crate::sources::own_root());
        let _ = std::fs::write(cache_path(), bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_page_gives_the_card() {
        let user = r#"{"user":{"id":17397924,"username":"NaumRedlo","country_code":"RU","country":{"code":"RU","name":"Russian Federation"},"avatar_url":"https://a.ppy.sh/17397924?1.jpeg","cover":{"url":"https://assets.ppy.sh/c.png"},"is_online":false,"is_supporter":true,"join_date":"2020-06-14T20:45:17+00:00","last_visit":"2026-09-23T20:10:43+00:00","rank_history":{"mode":"osu","data":[53724,52555,52567]},"statistics":{"pp":6396.01,"global_rank":52567,"country_rank":5101,"hit_accuracy":98.5614,"play_count":32519,"play_time":2333104,"ranked_score":15269370470,"total_score":96813980585,"total_hits":9607121,"maximum_combo":3971,"replays_watched_by_others":3,"grade_counts":{"ss":18,"ssh":2,"s":306,"sh":27,"a":644},"level":{"current":100,"progress":69}}}}"#;
        let page = format!("<div class=\"js-react\" data-initial-data=\"{}\"></div>", user.replace('"', "&quot;"));
        let card = card_from_page(&page).expect("a card");
        assert_eq!(card.username, "NaumRedlo");
        assert_eq!(card.osu_id, 17_397_924.0);
        assert_eq!(card.global_rank, 52_567.0);
        assert_eq!(card.country_rank, 5_101.0);
        assert_eq!(card.level, 100.0);
        assert_eq!(card.level_progress, 69.0);
        assert_eq!(card.grade_counts.ssh, 2.0);
        assert_eq!(card.rank_history, vec![53_724.0, 52_555.0, 52_567.0]);
        assert_eq!(card.country_name, "Russian Federation");
        assert!(card.avatar_url.starts_with("https://a.ppy.sh/"));
    }

    #[test]
    fn best_scores_read_with_their_hits() {
        let json = r#"[{"pp":386.125,"accuracy":0.996479,"rank":"S","mods":[{"acronym":"CL"},{"acronym":"HD"}],"max_combo":1705,"statistics":{"ok":9,"great":1695},"beatmap_id":1494828,"ended_at":"2026-05-11T13:10:05Z","beatmap":{"version":"Grace","difficulty_rating":6.6731,"max_combo":1710,"bpm":180,"total_length":320},"beatmapset":{"artist":"yaseta","title":"Bluenation","creator":"Meg","id":707032}}]"#;
        let scores = scores_from(json);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].mods, "HD");
        assert_eq!(scores[0].great, 1_695.0);
        assert_eq!(scores[0].ok, 9.0);
        assert_eq!((scores[0].bpm, scores[0].length, scores[0].played.as_str()), (180.0, 320.0, "2026-05-11T13:10:05Z"));
        assert!((scores[0].accuracy - 99.6479).abs() < 1e-6);
        assert_eq!(scores[0].cover().as_deref(), Some("https://assets.ppy.sh/beatmaps/707032/covers/cover.jpg"));
    }
}
