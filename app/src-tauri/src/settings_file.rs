//! The settings file on disk: told apart when missing or broken, written
//! atomically (temporary file, then rename) and readable only by the user.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde_json::Value;

#[derive(Debug)]
pub enum Loaded {
    /// No file yet: first start.
    Missing,
    Stored(Value),
    /// The file exists but cannot be read or is not JSON.
    Unreadable(String),
}

pub fn read(path: &Path) -> Loaded {
    match fs::read(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded::Missing,
        Err(e) => Loaded::Unreadable(e.to_string()),
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => Loaded::Stored(value),
            Err(e) => Loaded::Unreadable(format!("not valid JSON: {e}")),
        },
    }
}

/// Renames a broken file to `<name>.broken-<stamp>`; returns the new path.
pub fn set_aside(path: &Path, stamp: &str) -> io::Result<PathBuf> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".broken-{stamp}"));
    let kept = path.with_file_name(name);
    fs::rename(path, &kept)?;
    Ok(kept)
}

/// Replaces the file with `value`: a crash or a full disk leaves either the
/// old file or the new one, never a half-written one.
pub fn write(path: &Path, value: &Value) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("settings path has no folder"))?;
    fs::create_dir_all(dir)?;
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        // `mode` applies only to a new file; an old temporary may be wider.
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hutarka-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn missing_file_is_missing() {
        let dir = temp_dir("missing");
        assert!(matches!(read(&dir.join("settings.json")), Loaded::Missing));
    }

    #[test]
    fn broken_file_is_unreadable_not_missing() {
        let dir = temp_dir("broken");
        let path = dir.join("settings.json");
        fs::write(&path, b"{\"settings\": {\"api_key\": \"sk-").expect("write");
        assert!(matches!(read(&path), Loaded::Unreadable(_)));
    }

    #[test]
    fn set_aside_keeps_the_broken_bytes() {
        let dir = temp_dir("aside");
        let path = dir.join("settings.json");
        fs::write(&path, b"{broken").expect("write");
        let kept = set_aside(&path, "20261128-120000").expect("rename");
        assert_eq!(kept, dir.join("settings.json.broken-20261128-120000"));
        assert_eq!(fs::read(&kept).expect("read"), b"{broken");
        assert!(!path.exists());
    }

    #[test]
    fn write_then_read_round_trips_and_is_private() {
        let dir = temp_dir("write");
        let path = dir.join("sub").join("settings.json");
        let value = json!({"settings": {"api_key": "k", "explain": true}});
        write(&path, &value).expect("write");
        match read(&path) {
            Loaded::Stored(v) => assert_eq!(v, value),
            other => panic!("{other:?}"),
        }
        let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!dir.join("sub").join("settings.json.tmp").exists());
    }
}
