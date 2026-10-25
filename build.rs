use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    const PATH: &str = "themes/dracula.json";
    println!("cargo:rerun-if-changed={PATH}");
    let source = fs::read_to_string(PATH).expect("read bundled Dracula theme");
    let theme: serde_json::Value = serde_json::from_str(&source).expect("valid Dracula theme JSON");
    let mut colors = BTreeMap::new();
    for section in ["colors", "accents"] {
        for (name, value) in theme[section].as_object().expect("theme color object") {
            let hex = value.as_str().expect("color must be a hex string");
            assert!(
                hex.len() == 7
                    && hex.starts_with('#')
                    && hex[1..].bytes().all(|c| c.is_ascii_hexdigit()),
                "invalid color {name}: expected #RRGGBB"
            );
            let rgb = u32::from_str_radix(&hex[1..], 16).expect("validated hex color");
            assert!(
                colors.insert(name.as_str(), rgb).is_none(),
                "duplicate color {name}"
            );
        }
    }
    let mut generated = String::from("// Generated from themes/dracula.json; do not edit.\n");
    let name = theme["name"].as_str().expect("theme name");
    generated.push_str(&format!("pub const NAME: &str = {name:?};\n"));
    for (role, name) in theme["roles"].as_object().expect("theme roles object") {
        assert!(
            !role.is_empty() && role.bytes().all(|c| c.is_ascii_uppercase() || c == b'_'),
            "invalid theme role {role}"
        );
        let name = name.as_str().expect("role must reference a color name");
        let rgb = colors
            .get(name)
            .unwrap_or_else(|| panic!("unknown color {name} for {role}"));
        generated.push_str(&format!(
            "pub const {role}: Color = Color::from_rgb8({}, {}, {});\n",
            (rgb >> 16) & 255,
            (rgb >> 8) & 255,
            rgb & 255
        ));
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    fs::write(output.join("dracula.rs"), generated).expect("write compiled Dracula colors");
}
