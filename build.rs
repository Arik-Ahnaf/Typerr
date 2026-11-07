use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    for id in ["dracula", "gruvbox", "vscode"] {
        compile_theme(id, &output);
    }
    println!("cargo:rerun-if-changed=assets/icons/typerr-logo.ico");
    println!("cargo:rerun-if-changed=packaging/windows/typerr.rc");
    println!("cargo:rerun-if-env-changed=LLVM_RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = env::var("CARGO_PKG_VERSION").expect("package version");
        let parts: Vec<_> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "release version must be major.minor.patch");
        let numeric = format!("{},{},{},0", parts[0], parts[1], parts[2]);
        let icon = std::path::Path::new("assets/icons/typerr-logo.ico")
            .canonicalize()
            .expect("icon path");
        let source = format!(
            r#"1 ICON "{}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "FileDescription", "Typerr"
   VALUE "FileVersion", "{version}"
   VALUE "ProductVersion", "{version}"
   VALUE "ProductName", "Typerr"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#,
            icon.display().to_string().replace('\\', "/")
        );
        let rc = output.join("typerr.rc");
        fs::write(&rc, source).expect("write version resource");
        let resource = output.join("typerr.res");
        let compiler = env::var_os("LLVM_RC").unwrap_or_else(|| "llvm-rc".into());
        let status = std::process::Command::new(compiler)
            .arg("/FO")
            .arg(&resource)
            .arg(&rc)
            .status()
            .expect("run llvm-rc to embed the Windows icon (see BUILD.md)");
        assert!(status.success(), "compile Windows icon resource");
        println!("cargo:rustc-link-arg-bin=typerr={}", resource.display());
    }
}

fn compile_theme(id: &str, output: &std::path::Path) {
    let path = format!("themes/{id}.json");
    println!("cargo:rerun-if-changed={path}");
    let source = fs::read_to_string(&path).expect("read bundled theme");
    let theme: serde_json::Value = serde_json::from_str(&source).expect("valid bundled theme JSON");
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
    let mut generated = String::from("// Generated from bundled theme JSON; do not edit.\n");
    let name = theme["name"].as_str().expect("theme name");
    generated.push_str(&format!(
        "pub const PALETTE: Palette = Palette {{ name: {name:?},\n"
    ));
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
            "{}: Color::from_rgb8({}, {}, {}),\n",
            role.to_ascii_lowercase(),
            (rgb >> 16) & 255,
            (rgb >> 8) & 255,
            rgb & 255
        ));
    }
    generated.push_str("};\n");
    fs::write(output.join(format!("{id}.rs")), generated).expect("write compiled bundled colors");
}
