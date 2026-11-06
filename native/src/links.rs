use reqwest::Url;

const WEB: [&str; 2] = ["https", "http"];

pub fn checked(link: &str) -> Option<String> {
    let link = link.trim();
    if link.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(link).ok()?;
    let fine = match url.scheme() {
        scheme if WEB.contains(&scheme) => url.host_str().is_some_and(|host| !host.is_empty()),
        "mailto" => !url.path().is_empty(),
        "osu" => url.host_str().is_some_and(|host| !host.is_empty()),
        _ => false,
    };
    fine.then(|| url.to_string())
}

pub fn open(link: &str) -> bool {
    match checked(link) {
        Some(link) => open::that_detached(link).is_ok(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::checked;

    #[test]
    fn web_mail_and_osu_links_pass() {
        assert_eq!(checked("https://osu.ppy.sh/beatmapsets/1#osu/2").as_deref(), Some("https://osu.ppy.sh/beatmapsets/1#osu/2"));
        assert!(checked("http://localhost:8080/render/pair/AB/osu").is_some());
        assert!(checked("  HTTPS://t.me/OneNineEightFourGlobalBot?start=pair-1  ").is_some());
        assert!(checked("mailto:someone@example.com").is_some());
        assert_eq!(checked("osu://b/129891").as_deref(), Some("osu://b/129891"));
    }

    #[test]
    fn other_schemes_and_broken_links_are_refused() {
        for link in [
            "file:///etc/passwd",
            "file:///Applications/Calculator.app",
            "javascript:alert(1)",
            "smb://server/share",
            "ftp://example.com/x",
            "x-apple.systempreferences:com.apple.preference.security",
            "data:text/html,hi",
            "vscode://file/etc/hosts",
            "/Users/someone/Desktop",
            "C:\\Windows\\System32\\calc.exe",
            "https://",
            "osu://",
            "mailto:",
            "https://example.com/\u{0}hidden",
            "",
        ] {
            assert!(checked(link).is_none(), "{link}");
        }
    }

    #[test]
    fn only_local_places_are_opened_past_the_check() {
        let local = ["out", "self.settings.renders_dir()", "crate::sources::own_root().join(\"Songs\")", "crate::sources::own_root()", "crate::ffmpeg::own_dir()", "root", "folder", "&path", "path"];
        let mut stack = vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")];
        let mut seen = 0;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if !name.ends_with(".rs") || name == "links.rs" {
                    continue;
                }
                for line in std::fs::read_to_string(&path).unwrap().lines() {
                    for (at, _) in line.match_indices("open::that") {
                        let rest = &line[at..];
                        let Some(start) = rest.find('(') else { continue };
                        let mut depth = 0;
                        let end = rest[start..].char_indices().find_map(|(offset, c)| {
                            depth += match c { '(' => 1, ')' => -1, _ => 0 };
                            (depth == 0).then_some(start + offset)
                        });
                        let argument = rest[start + 1..end.unwrap()].trim();
                        assert!(local.iter().any(|known| argument == *known || argument == format!("&{known}")), "{name}: open::that({argument}) must go through links::open");
                        seen += 1;
                    }
                }
            }
        }
        assert!(seen > 0);
    }
}
