//! XDG registration with correct config paths and quoted executable names.
//! The plugin's Linux implementation hardcodes ~/.config and an unquoted Exec.
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tauri::Manager;

fn entry_path() -> Result<PathBuf, String> {
    directories::BaseDirs::new()
        .map(|dirs| {
            dirs.config_dir()
                .join("autostart/org.intelligentmusicplayer.app.desktop")
        })
        .ok_or_else(|| "Cannot find your configuration directory".into())
}

fn read_enabled(path: &Path) -> io::Result<bool> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    let mut in_entry = false;
    let mut hidden = false;
    let mut desktop_enabled = true;
    let mut command = false;
    for line in data.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Hidden" => hidden = value.trim() == "true",
                "X-GNOME-Autostart-enabled" => desktop_enabled = value.trim() != "false",
                "Exec" => command = !value.trim().is_empty(),
                _ => {}
            }
        }
    }
    Ok(command && !hidden && desktop_enabled)
}

fn desktop_entry(executable: &Path) -> Result<String, String> {
    let path = executable
        .to_str()
        .ok_or("The application path is not valid UTF-8")?;
    if !executable.is_absolute() || path.contains(['\n', '\r', '\0', '=']) {
        return Err("The application path cannot be used for login startup".into());
    }
    // Desktop Entry string escaping is applied before Exec quoting.
    let mut quoted = String::new();
    for ch in path.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\\\\\"),
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(ch);
            }
            '%' => quoted.push_str("%%"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(ch),
        }
    }
    Ok(format!("[Desktop Entry]\nType=Application\nName=Kaze\nExec=\"{quoted}\" {}\nIcon=org.intelligentmusicplayer.app\nTerminal=false\nStartupNotify=false\n", super::LOGIN_ARG))
}

fn write_enabled(path: &Path, executable: &Path, enabled: bool) -> Result<(), String> {
    if !enabled {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    let data = desktop_entry(executable)?;
    fs::create_dir_all(path.parent().ok_or("Invalid autostart path")?)
        .map_err(|e| e.to_string())?;
    // Replace atomically so a failed write cannot truncate an existing entry.
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = fs::write(&temporary, data).and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e| e.to_string())
}

pub fn is_enabled() -> Result<bool, String> {
    read_enabled(&entry_path()?).map_err(|e| e.to_string())
}

pub fn set_enabled(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    // AppImage mounts are temporary; always register the original portable file.
    let executable = app
        .env()
        .appimage
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_exe)
        .map_err(|e| e.to_string())?;
    write_enabled(&entry_path()?, &executable, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_roundtrip_and_external_disable() {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("autostart/kaze.desktop");
        let executable = Path::new("/opt/Kaze Music/Kaze.AppImage");
        assert!(!read_enabled(&entry).unwrap());
        write_enabled(&entry, executable, true).unwrap();
        assert!(read_enabled(&entry).unwrap());
        assert!(fs::read_to_string(&entry)
            .unwrap()
            .contains("Exec=\"/opt/Kaze Music/Kaze.AppImage\" --autostart"));
        fs::write(
            &entry,
            "[Desktop Entry]\nExec=kaze --autostart\nHidden=true\n",
        )
        .unwrap();
        assert!(!read_enabled(&entry).unwrap());
        fs::write(
            &entry,
            "[Desktop Entry]\nExec=kaze --autostart\nX-GNOME-Autostart-enabled=false\n",
        )
        .unwrap();
        assert!(!read_enabled(&entry).unwrap());
        write_enabled(&entry, executable, false).unwrap();
        write_enabled(&entry, executable, false).unwrap();
        assert!(!read_enabled(&entry).unwrap());
    }

    #[test]
    fn executable_escaping_and_write_failure() {
        let data = desktop_entry(Path::new("/opt/Kaze $music`\"\\100%.AppImage")).unwrap();
        assert!(data.contains(r#"Exec="/opt/Kaze \\$music\\`\\"\\\\100%%.AppImage" --autostart"#));
        assert!(desktop_entry(Path::new("/opt/bad\npath")).is_err());
        let root = tempfile::tempdir().unwrap();
        let blocker = root.path().join("file");
        fs::write(&blocker, "unchanged").unwrap();
        assert!(write_enabled(
            &blocker.join("entry.desktop"),
            Path::new("/usr/bin/kaze"),
            true
        )
        .is_err());
        assert_eq!(fs::read_to_string(blocker).unwrap(), "unchanged");
    }
}
