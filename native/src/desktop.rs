use std::path::Path;

pub fn reveal(path: &Path) -> Result<(), String> {
    let path = std::path::absolute(path).map_err(|e| e.to_string())?;
    std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if path.is_dir() {
        return open_folder(&path);
    }
    #[cfg(target_os = "macos")]
    if crate::checks::quiet("open").arg("-R").arg(&path).status().is_ok_and(|s| s.success()) {
        return Ok(());
    }
    #[cfg(windows)]
    return open_folder(path.parent().unwrap_or(Path::new(".")));
    #[cfg(target_os = "linux")]
    if let Ok(uri) = reqwest::Url::from_file_path(&path) {
        let result = crate::checks::quiet("dbus-send")
            .args(["--session", "--print-reply", "--reply-timeout=1500", "--dest=org.freedesktop.FileManager1", "/org/freedesktop/FileManager1", "org.freedesktop.FileManager1.ShowItems"])
            .arg(format!("array:string:{uri}"))
            .arg("string:")
            .output();
        if result.is_ok_and(|out| out.status.success()) {
            return Ok(());
        }
    }
    #[cfg(not(windows))]
    open_folder(path.parent().unwrap_or(Path::new(".")))
}

fn open_folder(path: &Path) -> Result<(), String> {
    let result = open::that(path);
    #[cfg(target_os = "linux")]
    if result.is_err() && crate::checks::quiet("gio").arg("open").arg(path).status().is_ok_and(|s| s.success()) {
        return Ok(());
    }
    result.map_err(|e| e.to_string())
}

pub const APP_ID: &str = "dossier";

pub fn window(settings: iced::window::Settings) -> iced::window::Settings {
    #[cfg(target_os = "linux")]
    let settings = iced::window::Settings {
        platform_specific: iced::window::settings::PlatformSpecific { application_id: APP_ID.to_owned(), ..Default::default() },
        ..settings
    };
    settings
}

pub fn entry(exe: &Path, icon: &Path) -> String {
    let quoted = |path: &Path| {
        let mut out = String::from("\"");
        for c in path.to_string_lossy().chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push_str("\\\\");
            }
            out.push(c);
        }
        out.push('"');
        out
    };
    format!(
        "[Desktop Entry]\nType=Application\nName=Dossier\nComment=osu! replays, videos and pools\nExec={}\nIcon={}\nTerminal=false\nCategories=Game;\nStartupWMClass={APP_ID}\n",
        quoted(exe),
        icon.to_string_lossy(),
    )
}

#[cfg(target_os = "linux")]
pub fn register() -> Result<(), String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|dir| !dir.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/share")))
        .ok_or("no home folder")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let icon = data.join("icons/hicolor/512x512/apps").join(format!("{APP_ID}.png"));
    let picture: &[u8] = include_bytes!("../assets/icon/dossier-512.png");
    if std::fs::read(&icon).ok().as_deref() != Some(picture) {
        std::fs::create_dir_all(icon.parent().ok_or("no icon folder")?).map_err(|e| e.to_string())?;
        std::fs::write(&icon, picture).map_err(|e| e.to_string())?;
    }
    let file = data.join("applications").join(format!("{APP_ID}.desktop"));
    let text = entry(&exe, &icon);
    if std::fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
        std::fs::create_dir_all(file.parent().ok_or("no applications folder")?).map_err(|e| e.to_string())?;
        std::fs::write(&file, text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn register() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_window_class_and_quotes_the_program() {
        let text = entry(Path::new("/home/a b/Dossier $1/dossier"), Path::new("/home/a b/.local/share/icons/hicolor/512x512/apps/dossier.png"));
        assert!(text.starts_with("[Desktop Entry]\n"));
        assert!(text.contains("\nStartupWMClass=dossier\n"));
        assert!(text.contains("\nExec=\"/home/a b/Dossier \\\\$1/dossier\"\n"));
        assert!(text.contains("\nIcon=/home/a b/.local/share/icons/hicolor/512x512/apps/dossier.png\n"));
    }
}
