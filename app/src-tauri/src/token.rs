//! The native Device token: Secret Service keyring first, else a 0600 file under `$XDG_DATA_HOME/orbit`.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub server: String,
    pub device: String,
    pub token: String,
}

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// `$XDG_DATA_HOME/orbit`, else `~/.local/share/orbit`, else `%APPDATA%\orbit` (Windows).
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("orbit")
}

fn entry() -> keyring::Result<keyring::Entry> {
    keyring::Entry::new("orbit", "desktop-device")
}

pub fn load() -> Option<Stored> {
    if let Some(s) = entry()
        .and_then(|e| e.get_password())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        return Some(s);
    }
    file_load(&data_dir())
}

pub fn save(s: &Stored) -> Res<()> {
    let json = serde_json::to_string(s)?;
    if entry().and_then(|e| e.set_password(&json)).is_ok() {
        // Don't leave an older copy in the fallback file.
        file_clear(&data_dir());
        return Ok(());
    }
    file_save(&data_dir(), s)
}

pub fn clear() {
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
    file_clear(&data_dir());
}

fn file(dir: &Path) -> PathBuf {
    dir.join("token")
}

pub fn file_load(dir: &Path) -> Option<Stored> {
    serde_json::from_str(&std::fs::read_to_string(file(dir)).ok()?).ok()
}

pub fn file_save(dir: &Path, s: &Stored) -> Res<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut f = options.open(file(dir))?;
    // An existing file keeps its old mode through `open`. Windows: the profile directory's ACL guards it.
    #[cfg(unix)]
    f.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    f.write_all(serde_json::to_string(s)?.as_bytes())?;
    Ok(())
}

pub fn file_clear(dir: &Path) {
    let _ = std::fs::remove_file(file(dir));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn stored() -> Stored {
        Stored {
            server: "http://x".into(),
            device: "d".into(),
            token: "t".into(),
        }
    }

    #[test]
    fn file_fallback_round_trips_with_mode_0600() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(file_load(dir.path()), None);
        file_save(dir.path(), &stored()).unwrap();
        assert_eq!(file_load(dir.path()), Some(stored()));
        let mode = std::fs::metadata(dir.path().join("token"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        file_clear(dir.path());
        assert_eq!(file_load(dir.path()), None);
    }

    #[test]
    fn overwriting_tightens_a_loose_file() {
        let dir = tempfile::tempdir().unwrap();
        file_save(dir.path(), &stored()).unwrap();
        let p = dir.path().join("token");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        file_save(dir.path(), &stored()).unwrap();
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
