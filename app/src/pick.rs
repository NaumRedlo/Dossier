//! Choosing a folder with the system's own dialog.
//!
//! Typing a path is the thing people get wrong first: a stray space, the wrong
//! slash, a folder they meant but did not name. Every system already has a
//! chooser; this asks for it in the one line each of them spells it in, rather
//! than pulling in a plugin and its permission file for a single call.

use std::process::Command;

/// AppleScript has no escape for a quote inside a string except a backslash,
/// and a backslash of its own has to be doubled. The prompt is ours, but a
/// prompt is exactly the sort of thing somebody later builds out of a folder
/// name.
fn quoted(text: &str) -> String {
    let inner: String = text
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            other => vec![other],
        })
        .collect();
    format!("\"{inner}\"")
}

/// Ask for a folder. `Ok(None)` means the dialog was closed without one, which
/// is an answer and not a failure.
pub fn folder(prompt: &str) -> Result<Option<String>, String> {
    let said = if cfg!(target_os = "macos") {
        Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "POSIX path of (choose folder with prompt {})",
                quoted(prompt)
            ))
            .output()
    } else if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "$f=(New-Object -ComObject Shell.Application).BrowseForFolder(0,{},0); \
                     if($f){{$f.Self.Path}}",
                    quoted(prompt)
                ),
            ])
            .output()
    } else {
        // Whichever of the two desktops is installed. Neither is guaranteed,
        // which is why the field beside the button still takes a typed path.
        let zenity = Command::new("zenity")
            .args(["--file-selection", "--directory", "--title", prompt])
            .output();
        match zenity {
            Ok(done) => Ok(done),
            Err(_) => Command::new("kdialog")
                .args(["--getexistingdirectory", ".", "--title", prompt])
                .output(),
        }
    };

    let done = said.map_err(|why| format!("окно выбора папки не открылось: {why}"))?;
    let path = String::from_utf8_lossy(&done.stdout).trim().to_owned();
    if path.is_empty() {
        // Cancelled, or no chooser on this machine. Both end the same way for
        // the person at the keyboard: the field is left as it was.
        return Ok(None);
    }
    Ok(Some(path.trim_end_matches('/').to_owned()))
}

/// Ask for one file. The filter is a bare extension, without a dot.
pub fn file(prompt: &str, extension: &str) -> Result<Option<String>, String> {
    let said = if cfg!(target_os = "macos") {
        Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "POSIX path of (choose file of type {{{}}} with prompt {})",
                quoted(extension),
                quoted(prompt)
            ))
            .output()
    } else if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Windows.Forms; \
                     $d=New-Object System.Windows.Forms.OpenFileDialog; \
                     $d.Filter={}; if($d.ShowDialog() -eq 'OK'){{$d.FileName}}",
                    quoted(&format!("*.{extension}|*.{extension}"))
                ),
            ])
            .output()
    } else {
        let zenity = Command::new("zenity")
            .args([
                "--file-selection",
                "--title",
                prompt,
                "--file-filter",
                &format!("*.{extension}"),
            ])
            .output();
        match zenity {
            Ok(done) => Ok(done),
            Err(_) => Command::new("kdialog")
                .args(["--getopenfilename", ".", &format!("*.{extension}")])
                .output(),
        }
    };

    let done = said.map_err(|why| format!("окно выбора файла не открылось: {why}"))?;
    let path = String::from_utf8_lossy(&done.stdout).trim().to_owned();
    Ok(if path.is_empty() { None } else { Some(path) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prompt_is_quoted_and_cannot_end_its_own_string() {
        assert_eq!(quoted("Папка карт"), "\"Папка карт\"");
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
        assert_eq!(quoted("a\\b"), "\"a\\\\b\"");
        assert_eq!(quoted("one\ntwo"), "\"onetwo\"");
    }
}
