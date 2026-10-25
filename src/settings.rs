//! Startup configuration; missing or invalid settings retain usable defaults.
use std::{fs, path::Path};

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
        // Prefer the working directory for cargo run, then a portable executable's directory.
        let local = Path::new("settings.json");
        let path = if local.exists() {
            local.to_path_buf()
        } else if let Ok(executable) = std::env::current_exe() {
            executable.with_file_name("settings.json")
        } else {
            return Self::default();
        };
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
