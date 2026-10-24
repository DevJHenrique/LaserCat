// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    domain::{MachineProfile, Project},
    i18n::{tr, Language},
    limits::MAX_ASSET_BYTES,
    trf,
};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub const PROJECT_EXTENSION: &str = "lcc";

/// UI saves use the current extension; the low-level archive API honors its exact path.
pub fn project_save_path(mut path: PathBuf) -> PathBuf {
    path.set_extension(PROJECT_EXTENSION);
    path
}

pub fn save(path: &Path, p: &Project) -> Result<(), String> {
    p.validate()?;
    let manifest = serde_json::to_vec(p).map_err(|e| e.to_string())?;
    if manifest.len() > 64 * 1024 * 1024
        || p.assets
            .iter()
            .any(|a| a.bytes.len() as u64 > MAX_ASSET_BYTES)
        || p.assets.iter().map(|a| a.bytes.len()).sum::<usize>() + manifest.len()
            > 512 * 1024 * 1024
    {
        return Err(tr(
            "The project exceeds the size limits; remove assets or simplify the files.",
        )
        .into());
    }
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    {
        let mut zip = ZipWriter::new(temp.as_file_mut());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&manifest).map_err(|e| e.to_string())?;
        for a in &p.assets {
            zip.start_file(format!("assets/{}", a.id), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&a.bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    temp.as_file_mut().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn limited_read(mut reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut data = vec![];
    reader
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err(tr("The project exceeds the size limits.").into());
    }
    Ok(data)
}
pub fn load(path: &Path) -> Result<Project, String> {
    let mut zip =
        ZipArchive::new(File::open(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let manifest = limited_read(
        zip.by_name("manifest.json").map_err(|e| e.to_string())?,
        64 * 1024 * 1024,
    )?;
    let mut p: Project = serde_json::from_slice(&manifest).map_err(|e| e.to_string())?;
    p.validate()?;
    let mut total = manifest.len() as u64;
    for a in &mut p.assets {
        let a = std::sync::Arc::make_mut(a);
        a.bytes = std::sync::Arc::new(limited_read(
            zip.by_name(&format!("assets/{}", a.id))
                .map_err(|e| e.to_string())?,
            MAX_ASSET_BYTES,
        )?);
        total += a.bytes.len() as u64;
        if total > 512 * 1024 * 1024 {
            return Err(tr("The project exceeds 512 MB.").into());
        }
    }
    Ok(p)
}
pub fn recovery_path() -> PathBuf {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LaserCat");
    dir.join("recovery.lcc")
}

/// Removes the current recovery copy after a clean save. Legacy recovery files are never touched.
pub fn discard_recovery(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

pub fn device_path() -> PathBuf {
    recovery_path().with_file_name("device.json")
}

fn legacy_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LaserStudio")
}

pub fn legacy_device_path() -> PathBuf {
    legacy_directory().join("device.json")
}

pub fn recovery_source() -> Option<PathBuf> {
    [recovery_path(), legacy_directory().join("recovery.laser")]
        .into_iter()
        .find(|p| p.exists())
}

pub fn load_device_with_legacy(
    primary: &Path,
    legacy: &Path,
) -> Result<Option<MachineProfile>, String> {
    match load_device(primary)? {
        Some(profile) => Ok(Some(profile)),
        None => load_device(legacy),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DeviceSettings {
    version: u32,
    machine: MachineProfile,
}

pub fn save_device(path: &Path, machine: &MachineProfile) -> Result<(), String> {
    if machine.configured {
        machine.validate()?;
    }
    let bytes = serde_json::to_vec_pretty(&DeviceSettings {
        version: 1,
        machine: machine.clone(),
    })
    .map_err(|e| e.to_string())?;
    write_settings(path, &bytes)
}

/// Atomically replaces a small local settings file, creating its directory.
fn write_settings(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or(tr("Invalid settings directory"))?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file_mut().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Local interface preferences. Like the device profile, they never travel with projects.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub language: Language,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PreferencesFile {
    version: u32,
    #[serde(flatten)]
    preferences: Preferences,
}

pub fn preferences_path() -> PathBuf {
    recovery_path().with_file_name("preferences.json")
}

pub fn save_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&PreferencesFile {
        version: 1,
        preferences: preferences.clone(),
    })
    .map_err(|e| e.to_string())?;
    write_settings(path, &bytes)
}

/// Missing preferences mean defaults (English). Invalid files are reported, not hidden.
pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Preferences::default()),
        Err(e) => return Err(e.to_string()),
    };
    let settings: PreferencesFile = serde_json::from_slice(&limited_read(file, 64 * 1024)?)
        .map_err(|e| trf!("Invalid preferences: {error}", error = e))?;
    if settings.version != 1 {
        return Err(tr("Incompatible preferences version.").into());
    }
    Ok(settings.preferences)
}

pub fn load_device(path: &Path) -> Result<Option<MachineProfile>, String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let settings: DeviceSettings = serde_json::from_slice(&limited_read(file, 64 * 1024)?)
        .map_err(|e| trf!("Invalid device settings: {error}", error = e))?;
    if settings.version != 1 {
        return Err(tr("Incompatible device settings version.").into());
    }
    if settings.machine.configured {
        settings.machine.validate()?;
    }
    Ok(Some(settings.machine))
}

#[cfg(test)]
mod device_tests {
    use super::*;
    #[test]
    fn device_survives_restart_and_atomic_replacement() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("settings/device.json");
        assert!(load_device(&path).unwrap().is_none());
        let mut profile = MachineProfile {
            configured: true,
            width: 400.,
            height: 300.,
            port: "COM3".into(),
            invert_y: true,
            ..Default::default()
        };
        save_device(&path, &profile).unwrap();
        assert_eq!(load_device(&path).unwrap(), Some(profile.clone()));
        profile.baud = 230400;
        save_device(&path, &profile).unwrap();
        assert_eq!(load_device(&path).unwrap(), Some(profile));
        std::fs::write(&path, b"{broken}").unwrap();
        assert!(load_device(&path).is_err());
    }
    #[test]
    fn preferences_default_to_english_and_round_trip() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("settings/preferences.json");
        assert_eq!(load_preferences(&path).unwrap().language, Language::English);
        let preferences = Preferences {
            language: Language::Portuguese,
        };
        save_preferences(&path, &preferences).unwrap();
        assert_eq!(load_preferences(&path).unwrap(), preferences);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("\"pt-BR\""));
        std::fs::write(&path, br#"{"version":1,"language":"xx"}"#).unwrap();
        assert!(load_preferences(&path).is_err());
    }
}
