//! Startup configuration; missing or invalid settings retain usable defaults.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dracula,
    Gruvbox,
    VSCode,
}
impl Theme {
    pub const NAMES: [&str; 3] = ["Dracula", "Gruvbox", "VSCode"];
    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("dracula") {
            Some(Self::Dracula)
        } else if value.eq_ignore_ascii_case("gruvbox") {
            Some(Self::Gruvbox)
        } else if value.eq_ignore_ascii_case("vscode") {
            Some(Self::VSCode)
        } else {
            None
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Dracula => "Dracula",
            Self::Gruvbox => "Gruvbox",
            Self::VSCode => "VSCode",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub font_size: f32,
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            theme: Theme::default(),
        }
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
        let theme = object
            .get("theme")
            .and_then(serde_json::Value::as_str)
            .and_then(Theme::parse)
            .unwrap_or_default();
        let font_size = match object.get("font-size") {
            Some(value) => value.as_f64().ok_or("font-size must be a number")?,
            None => 16.0,
        };
        if !(8.0..=48.0).contains(&font_size) {
            return Err("font-size must be between 8 and 48 logical pixels".into());
        }
        Ok(Self {
            font_size: font_size as f32,
            theme,
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
    fn theme_selection_defaults_and_case_insensitive_names() {
        for name in ["Gruvbox", "gruvbox", "GRUVBOX"] {
            let settings = Settings::parse(&format!(r#"{{"theme":"{name}"}}"#)).unwrap();
            assert_eq!(settings.theme, Theme::Gruvbox);
            assert_eq!(settings.font_size, 16.);
        }
        for value in [r#""Dracula""#, r#""unknown""#, "null", "42"] {
            let settings =
                Settings::parse(&format!(r#"{{"theme":{value},"font-size":24}}"#)).unwrap();
            assert_eq!(settings.theme, Theme::Dracula);
            assert_eq!(settings.font_size, 24.);
        }
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
        self.save_patch(serde_json::json!({"name": name}))?;
        self.name = Some(name);
        Ok(())
    }

    pub fn parameters(&self) -> Result<Parameters, String> {
        let object = read_object(&self.path)?;
        let mut parameters = Parameters::default();
        if let Some(size) = object.get("font-size").and_then(serde_json::Value::as_f64)
            && (8.0..=48.0).contains(&size)
        {
            parameters.font_size = size as f32;
        }
        if let Some(value) = object
            .get("font-family")
            .and_then(serde_json::Value::as_str)
            && value == "Google Sans Code"
        {
            parameters.font_family = value.into();
        }
        if let Some(value) = object
            .get("caret-style")
            .and_then(serde_json::Value::as_str)
            && ["Line", "Block", "Underline"].contains(&value)
        {
            parameters.caret_style = value.into();
        }
        if let Some(value) = object.get("theme").and_then(serde_json::Value::as_str)
            && let Some(theme) = Theme::parse(value)
        {
            parameters.theme = theme.name().into();
        }
        if let Some(value) = object.get("sound").and_then(serde_json::Value::as_bool) {
            parameters.sound = value;
        }
        Ok(parameters)
    }

    pub fn save_parameters(&mut self, parameters: &Parameters) -> Result<(), String> {
        if !(8.0..=48.0).contains(&parameters.font_size)
            || parameters.font_family != "Google Sans Code"
            || !["Line", "Block", "Underline"].contains(&parameters.caret_style.as_str())
            || Theme::parse(&parameters.theme).is_none()
        {
            return Err("Invalid settings parameters.".into());
        }
        self.save_patch(serde_json::json!({
            "font-size": parameters.font_size, "font-family": parameters.font_family,
            "caret-style": parameters.caret_style, "theme": parameters.theme, "sound": parameters.sound,
        }))
    }

    fn save_patch(&mut self, patch: serde_json::Value) -> Result<(), String> {
        let patch = patch
            .as_object()
            .expect("internal settings patch is an object");
        let mut object = read_object(&self.path)?;
        object.extend(patch.clone());
        let encode = |object: &serde_json::Map<String, serde_json::Value>| {
            serde_json::to_vec_pretty(object).map_err(|e| e.to_string())
        };
        if let Err(error) = write_settings(&self.path, &encode(&object)?) {
            let fallback = self
                .fallback
                .as_ref()
                .filter(|p| **p != self.path)
                .ok_or_else(|| format!("Could not save settings: {error}"))?;
            // A fallback created since startup may contain newer unrelated keys.
            // Parse it before replacing it, just as we do the original file.
            object.extend(read_object(fallback)?);
            object.extend(patch.clone());
            write_settings(fallback, &encode(&object)?)
                .map_err(|e| format!("Could not save settings: {e}"))?;
            self.path = fallback.clone();
        }
        Ok(())
    }
}
/// Persisted Settings-page values. Only font size is currently applied at startup.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameters {
    pub font_size: f32,
    pub font_family: String,
    pub caret_style: String,
    pub theme: String,
    pub sound: bool,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            font_size: 16.,
            font_family: "Google Sans Code".into(),
            caret_style: "Line".into(),
            theme: "Dracula".into(),
            sound: false,
        }
    }
}
fn read_object(path: &Path) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let value: serde_json::Value = match fs::read_to_string(path) {
        Ok(source) => {
            serde_json::from_str(&source).map_err(|e| format!("Cannot read settings: {e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(format!("Cannot read settings: {e}")),
    };
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "Settings must be a JSON object.".into())
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

#[cfg(test)]
mod parameter_tests {
    use super::*;
    #[test]
    fn save_and_reload_all_parameters_preserves_name_and_external_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"name":"Alice","font-size":24.5,"future":{"enabled":true}}"#,
        )
        .unwrap();
        let mut profile = Profile::at(path.clone(), None);
        assert_eq!(profile.parameters().unwrap().font_size, 24.5);
        let parameters = Parameters {
            font_size: 32.,
            caret_style: "Underline".into(),
            theme: "Gruvbox".into(),
            sound: true,
            ..Parameters::default()
        };
        // Re-read at save time, rather than overwriting a stale startup snapshot.
        fs::write(
            &path,
            r#"{"name":"Alice","future":{"enabled":false},"external":42}"#,
        )
        .unwrap();
        profile.save_parameters(&parameters).unwrap();
        assert_eq!(
            Profile::at(path.clone(), None).parameters().unwrap(),
            parameters
        );
        assert_eq!(
            Settings::parse(&fs::read_to_string(&path).unwrap())
                .unwrap()
                .font_size,
            32.
        );
        profile.save("Bob").unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(value["name"], "Bob");
        assert_eq!(value["external"], 42);
        assert_eq!(value["future"]["enabled"], false);
        assert_eq!(value["sound"], true);
        assert_eq!(profile.parameters().unwrap(), parameters);
    }
    #[test]
    fn invalid_json_and_parameters_are_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        for source in ["{", "[]", "null"] {
            fs::write(&path, source).unwrap();
            let mut profile = Profile::at(path.clone(), None);
            assert!(profile.parameters().is_err());
            assert!(profile.save_parameters(&Parameters::default()).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
        }
        fs::write(&path, "{}").unwrap();
        let mut profile = Profile::at(path.clone(), None);
        for size in [0., 49., f32::NAN, f32::INFINITY] {
            assert!(
                profile
                    .save_parameters(&Parameters {
                        font_size: size,
                        ..Parameters::default()
                    })
                    .is_err()
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
        }
        assert!(
            profile
                .save_parameters(&Parameters {
                    caret_style: "unknown".into(),
                    ..Parameters::default()
                })
                .is_err()
        );
    }
    #[test]
    fn absent_and_unsupported_fields_have_independent_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let profile = Profile::at(path.clone(), None);
        assert_eq!(profile.parameters().unwrap(), Parameters::default());
        fs::write(path, r#"{"font-size":20,"font-family":"unknown","caret-style":"unknown","theme":"unknown","sound":true}"#).unwrap();
        assert_eq!(
            profile.parameters().unwrap(),
            Parameters {
                font_size: 20.,
                sound: true,
                ..Parameters::default()
            }
        );
    }
    #[cfg(unix)]
    #[test]
    fn parameter_fallback_preserves_keys_and_rejects_malformed_destination() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path().join("install");
        fs::create_dir(&install).unwrap();
        let source = install.join("settings.json");
        let fallback = dir.path().join("user/settings.json");
        fs::write(&source, r#"{"name":"Alice","keep":true}"#).unwrap();
        fs::create_dir(fallback.parent().unwrap()).unwrap();
        fs::write(&fallback, "{").unwrap();
        let mut profile = Profile::at(source.clone(), Some(fallback.clone()));
        fs::set_permissions(&install, fs::Permissions::from_mode(0o555)).unwrap();
        let invalid = profile.save_parameters(&Parameters::default());
        fs::set_permissions(&install, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(invalid.is_err());
        assert_eq!(fs::read_to_string(&fallback).unwrap(), "{");
        fs::write(&fallback, r#"{"extra":7}"#).unwrap();
        fs::set_permissions(&install, fs::Permissions::from_mode(0o555)).unwrap();
        let result = profile.save_parameters(&Parameters {
            sound: true,
            ..Parameters::default()
        });
        fs::set_permissions(&install, fs::Permissions::from_mode(0o755)).unwrap();
        result.unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(fallback).unwrap()).unwrap();
        assert_eq!(value["name"], "Alice");
        assert_eq!(value["keep"], true);
        assert_eq!(value["extra"], 7);
        assert_eq!(value["sound"], true);
        assert_eq!(
            fs::read_to_string(source).unwrap(),
            r#"{"name":"Alice","keep":true}"#
        );
    }
}
