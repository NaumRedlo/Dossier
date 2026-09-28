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
