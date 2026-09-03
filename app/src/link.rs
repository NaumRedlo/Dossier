//! Handing a link to whatever this system opens links with.
//!
//! Not `tauri-plugin-opener`: that is another dependency, another permission
//! file and another thing to keep in step across three systems, for a call each
//! of them already spells in one line.

/// The schemes that may be handed over.
///
/// Short on purpose. The window's content is ours, but `file://` would open
/// anything on this machine and a shell scheme would run it — and the day
/// somebody builds a link out of something a server said, this is the line that
/// will already have been drawn.
fn allowed(url: &str) -> bool {
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

/// Open a link in the browser, the mail client, whatever claims it.
pub fn open(url: &str) -> Result<(), String> {
    if !allowed(url) {
        return Err("такие ссылки не открываются".to_owned());
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        // The empty argument is `start`'s title, which it otherwise takes from
        // the URL and then has nothing left to open.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_links_pass() {
        assert!(allowed("https://github.com/NaumRedlo/Dossier"));
        assert!(allowed("mailto:someone@example.com?subject=hi"));
    }

    #[test]
    fn the_rest_do_not() {
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("/Users/somebody/secrets"));
        assert!(!allowed(" https://sneaky"));
    }

    #[test]
    fn a_refused_link_says_so_rather_than_opening() {
        assert!(open("file:///etc/passwd").is_err());
    }
}
