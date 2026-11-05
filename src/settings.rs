//! Startup configuration; missing or invalid settings retain usable defaults.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub font_size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { font_size: 16.0 }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();
        match fs::read_to_string(&path) {
            Ok(source) => Self::parse(&source).unwrap_or_else(|error| {
                eprintln!(
                    "Invalid {}: {error}; using default settings",
                    path.display()
                );
                Self::default()
            }),
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "Could not read {}: {error}; using default settings",
                        path.display()
                    );
                }
                Self::default()
            }
        }
    }

    fn parse(source: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
        let object = value.as_object().ok_or("expected a JSON object")?;
        let Some(value) = object.get("font-size") else {
            return Ok(Self::default());
        };
        let font_size = value.as_f64().ok_or("font-size must be a number")?;
        if !(8.0..=48.0).contains(&font_size) {
            return Err("font-size must be between 8 and 48 logical pixels".into());
        }
        Ok(Self {
            font_size: font_size as f32,
        })
    }

    pub fn line_height(self) -> f32 {
        self.font_size * 1.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_fractional_sizes() {
        assert_eq!(Settings::parse("{}").unwrap().font_size, 16.0);
        let settings = Settings::parse(r#"{"font-size": 24.5}"#).unwrap();
        assert_eq!(settings.font_size, 24.5);
        assert_eq!(settings.line_height(), 30.625);
    }

    #[test]
    fn rejects_malformed_and_unusable_sizes() {
        for source in [
            "{",
            "[]",
            r#"{"font-size": "20"}"#,
            r#"{"font-size": null}"#,
            r#"{"font-size": 0}"#,
            r#"{"font-size": -1}"#,
            r#"{"font-size": 49}"#,
        ] {
            assert!(Settings::parse(source).is_err(), "{source}");
        }
        for size in [8, 48] {
            assert!(Settings::parse(&format!(r#"{{"font-size": {size}}}"#)).is_ok());
        }
    }
}

/// User configuration overrides a bundled, potentially read-only settings file.
pub(crate) fn user_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    base.map(|p| p.join("typerr/settings.json"))
}
fn settings_path() -> PathBuf {
    if let Some(path) = user_settings_path().filter(|p| p.exists()) {
        return path;
    }
    let local = PathBuf::from("settings.json");
    if local.exists() {
        local
    } else {
        std::env::current_exe()
            .map(|p| p.with_file_name("settings.json"))
            .unwrap_or(local)
    }
}
pub fn valid_name(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty() && name.chars().count() <= 16 && !name.chars().any(char::is_control))
        .then(|| name.to_owned())
}
#[derive(Debug)]
pub struct Profile {
    pub name: Option<String>,
    path: PathBuf,
    fallback: Option<PathBuf>,
}
impl Profile {
    pub fn load() -> Self {
        Self::at(settings_path(), user_settings_path())
    }
    pub(crate) fn at(path: PathBuf, fallback: Option<PathBuf>) -> Self {
        let name = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("name").and_then(|n| n.as_str()).and_then(valid_name));
        Self {
            name,
            path,
            fallback,
        }
    }
    pub fn save(&mut self, name: &str) -> Result<(), String> {
        let name = valid_name(name).ok_or("Enter a name of 1–16 characters.")?;
        let mut value: serde_json::Value = match fs::read_to_string(&self.path) {
            Ok(source) => {
                serde_json::from_str(&source).map_err(|e| format!("Cannot read settings: {e}"))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
            Err(e) => return Err(format!("Cannot read settings: {e}")),
        };
        value
            .as_object_mut()
            .ok_or("Settings must be a JSON object.")?
            .insert("name".into(), name.clone().into());
        let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
        if let Err(error) = write_settings(&self.path, &bytes) {
            let fallback = self
                .fallback
                .as_ref()
                .filter(|p| **p != self.path)
                .ok_or_else(|| format!("Could not save name: {error}"))?;
            write_settings(fallback, &bytes).map_err(|e| format!("Could not save name: {e}"))?;
            self.path = fallback.clone();
        }
        self.name = Some(name);
        Ok(())
    }
}
fn write_settings(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[test]
    fn name_validation_and_roundtrip_preserve_other_settings() {
        for bad in ["", "   ", "abcdefghijklmnopq", "a\nb"] {
            assert!(valid_name(bad).is_none());
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"font-size":24,"future":{"enabled":true}}"#).unwrap();
        let mut profile = Profile::at(path.clone(), None);
        assert!(profile.name.is_none());
        profile.save("  Ålice 世界  ").unwrap();
        assert_eq!(
            Profile::at(path.clone(), None).name.as_deref(),
            Some("Ålice 世界")
        );
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(value["font-size"], 24);
        assert_eq!(value["future"]["enabled"], true);
        assert!(valid_name(&"é".repeat(16)).is_some());
        assert!(valid_name(&"é".repeat(17)).is_none());
    }
    #[test]
    fn failed_save_keeps_prompt_and_never_overwrites_malformed_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{").unwrap();
        let mut profile = Profile::at(path.clone(), None);
        assert!(profile.save("Alice").is_err());
        assert!(profile.name.is_none());
        assert_eq!(fs::read_to_string(path).unwrap(), "{");
    }
    #[test]
    fn absent_file_and_invalid_saved_names_require_onboarding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut profile = Profile::at(path.clone(), None);
        profile.save("abcdefghijklmnop").unwrap();
        assert_eq!(
            Profile::at(path.clone(), None).name.as_deref(),
            Some("abcdefghijklmnop")
        );
        fs::write(&path, r#"{"name":" "}"#).unwrap();
        assert!(Profile::at(path, None).name.is_none());
    }
}

#[cfg(all(test, unix))]
mod fallback_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn read_only_install_uses_user_settings_and_preserves_font() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path().join("install");
        fs::create_dir(&install).unwrap();
        let source = install.join("settings.json");
        fs::write(&source, r#"{"font-size":24}"#).unwrap();
        fs::set_permissions(&install, fs::Permissions::from_mode(0o555)).unwrap();
        let destination = dir.path().join("user/settings.json");
        let mut profile = Profile::at(source.clone(), Some(destination.clone()));
        let result = profile.save("Alice");
        fs::set_permissions(&install, fs::Permissions::from_mode(0o755)).unwrap();
        result.unwrap();
        assert_eq!(
            Profile::at(destination.clone(), None).name.as_deref(),
            Some("Alice")
        );
        assert_eq!(
            Settings::parse(&fs::read_to_string(destination).unwrap())
                .unwrap()
                .font_size,
            24.
        );
        assert_eq!(fs::read_to_string(source).unwrap(), r#"{"font-size":24}"#);
    }
}
