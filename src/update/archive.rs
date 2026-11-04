//! Extract only the portable package's known payload into a fresh directory.
use super::{Error, Result, release::Distribution};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
pub fn executable() -> &'static str {
    if cfg!(windows) {
        "typerr.exe"
    } else {
        "typerr"
    }
}
fn relative(name: &Path, root: &str) -> Result<PathBuf> {
    let text = name.to_str().ok_or("Non-UTF-8 archive path")?;
    if text.contains(['\\', ':']) {
        return Err("Unsafe archive path".into());
    }
    let parts: Vec<_> = name.components().collect();
    if parts.iter().any(|p| !matches!(p, Component::Normal(_)))
        || parts.first().is_none_or(|p| p.as_os_str() != root)
    {
        return Err("Unsafe archive path".into());
    }
    let path: PathBuf = parts.iter().skip(1).map(|p| p.as_os_str()).collect();
    if !path.as_os_str().is_empty() && !allowed(&path) {
        return Err("Unexpected file in update archive".into());
    }
    Ok(path)
}
pub fn allowed(path: &Path) -> bool {
    let Some(s) = path.to_str() else {
        return false;
    };
    let valid =
        path.components().all(|c| matches!(c, Component::Normal(_))) && !s.contains(['\\', ':']);
    valid
        && (matches!(
            s,
            "typerr"
                | "typerr.exe"
                | "settings.json"
                | "README.md"
                | "typerr.desktop"
                | "typerr-logo.svg"
                | "notices"
        ) || s.starts_with("notices/"))
}
struct Extractor<'a> {
    output: &'a Path,
    root: &'a str,
    seen: HashSet<String>,
    total: u64,
}
impl Extractor<'_> {
    fn entry(
        &mut self,
        name: &Path,
        directory: bool,
        size: u64,
        reader: &mut dyn Read,
    ) -> Result<()> {
        let rel = relative(name, self.root)?;
        if rel.as_os_str().is_empty() {
            if directory {
                return Ok(());
            } else {
                return Err("Invalid archive root".into());
            }
        }
        // Case folding also rejects collisions on Windows' case-insensitive filesystems.
        if !self.seen.insert(rel.to_string_lossy().to_lowercase()) || self.seen.len() > MAX_ENTRIES
        {
            return Err("Duplicate or excessive archive entries".into());
        }
        self.total = self
            .total
            .checked_add(size)
            .ok_or("Archive size overflow")?;
        if self.total > MAX_EXPANDED {
            return Err("Expanded archive is too large".into());
        }
        let dest = self.output.join(&rel);
        if directory {
            fs::create_dir_all(dest)?;
            return Ok(());
        }
        fs::create_dir_all(dest.parent().ok_or("Missing archive parent")?)?;
        let mut file = File::create_new(dest)?;
        let copied = std::io::copy(&mut reader.take(size + 1), &mut file)?;
        if copied != size {
            return Err("Invalid archive entry size".into());
        }
        file.flush()?;
        file.sync_all()?;
        Ok(())
    }
}
pub fn extract(archive: &Path, output: &Path, version: &str, kind: Distribution) -> Result<()> {
    fs::create_dir(output)?;
    let suffix = if kind == Distribution::Windows {
        "windows-x86_64"
    } else {
        "arch-linux-x86_64"
    };
    let root = format!("typerr-{version}-{suffix}");
    let mut extractor = Extractor {
        output,
        root: &root,
        seen: HashSet::new(),
        total: 0,
    };
    if kind == Distribution::Windows {
        let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
        if zip.len() > MAX_ENTRIES {
            return Err("Too many archive entries".into());
        }
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.unix_mode().is_some_and(|m| {
                m & 0o170000 != 0 && m & 0o170000 != 0o100000 && m & 0o170000 != 0o040000
            }) {
                return Err("Archive links are forbidden".into());
            }
            let name = PathBuf::from(entry.name());
            let dir = entry.is_dir();
            let size = entry.size();
            extractor.entry(&name, dir, size, &mut entry)?;
        }
    } else {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(archive)?));
        for entry in tar.entries()? {
            let mut entry = entry?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err("Archive links and special entries are forbidden".into());
            }
            let name = entry.path()?.into_owned();
            let size = entry.size();
            extractor.entry(&name, kind.is_dir(), size, &mut entry)?;
        }
    }
    for required in [executable(), "README.md", "settings.json"] {
        if !output.join(required).is_file() {
            return Err(format!("Missing package file {required}").into());
        }
    }
    if !output.join("notices").is_dir() {
        return Err("Missing dependency notices".into());
    }
    let binary = output.join(executable());
    verify_architecture(&binary, kind)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))?;
    }
    let result = output_timeout(
        hidden_command(&binary).arg("--version"),
        Duration::from_secs(10),
    )?;
    if !result.status.success()
        || String::from_utf8_lossy(&result.stdout).trim() != format!("Typerr {version}")
    {
        return Err("Downloaded executable version does not match the release".into());
    }
    Ok(())
}
fn verify_architecture(path: &Path, kind: Distribution) -> Result<()> {
    use std::io::{Seek, SeekFrom};
    let mut file = File::open(path)?;
    let mut header = [0; 64];
    file.read_exact(&mut header)?;
    if kind == Distribution::Windows {
        if &header[..2] != b"MZ" {
            return Err("Update is not a Windows executable".into());
        }
        let offset = u32::from_le_bytes(header[60..64].try_into()?);
        file.seek(SeekFrom::Start(offset as u64))?;
        let mut pe = [0; 6];
        file.read_exact(&mut pe)?;
        if &pe[..4] != b"PE\0\0" || u16::from_le_bytes([pe[4], pe[5]]) != 0x8664 {
            return Err("Update is not Windows x64".into());
        }
    } else if &header[..4] != b"\x7fELF"
        || header[4] != 2
        || header[5] != 1
        || u16::from_le_bytes([header[18], header[19]]) != 62
    {
        return Err("Update is not Linux x64".into());
    }
    Ok(())
}
pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command.stdin(Stdio::null());
    command
}
pub fn output_timeout(command: &mut Command, timeout: Duration) -> Result<std::process::Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    // Drain both pipes concurrently; cap retained output without deadlocking a verbose process.
    let drain = |reader: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let mut reader = reader;
            let mut buf = [0; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let keep = n.min((64 * 1024usize).saturating_sub(bytes.len()));
                bytes.extend_from_slice(&buf[..keep]);
            }
            bytes
        })
    };
    let out = drain(Box::new(child.stdout.take().ok_or("Missing stdout")?));
    let err = drain(Box::new(child.stderr.take().ok_or("Missing stderr")?));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::from("Update command timed out"));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    Ok(std::process::Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_and_limits() {
        for name in [
            "/root/typerr",
            "root/../typerr",
            "other/typerr",
            "root/notices/../../settings.json",
            "root/C:evil",
            "root/notices\\evil",
            "root/secret",
        ] {
            assert!(relative(Path::new(name), "root").is_err(), "{name}");
        }
        let temp = tempfile::tempdir().unwrap();
        let mut ex = Extractor {
            output: temp.path(),
            root: "root",
            seen: HashSet::new(),
            total: 0,
        };
        ex.entry(Path::new("root/README.md"), false, 2, &mut &b"ok"[..])
            .unwrap();
        assert!(
            ex.entry(Path::new("root/README.md"), false, 2, &mut &b"ok"[..])
                .is_err()
        );
        assert!(
            ex.entry(
                Path::new("root/notices/huge"),
                false,
                MAX_EXPANDED,
                &mut &b""[..]
            )
            .is_err()
        );
    }
    #[test]
    fn tar_links_and_wrong_executables_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.tar.gz");
        let gzip = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(gzip);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_link_name("/etc/passwd").unwrap();
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                "typerr-1.2.0-arch-linux-x86_64/typerr",
                std::io::empty(),
            )
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        assert!(
            extract(
                &path,
                &temp.path().join("output"),
                "1.2.0",
                Distribution::Linux
            )
            .is_err()
        );
        let fake = temp.path().join("binary");
        fs::write(&fake, [0u8; 64]).unwrap();
        assert!(verify_architecture(&fake, Distribution::Linux).is_err());
        assert!(verify_architecture(&fake, Distribution::Windows).is_err());
    }
    #[test]
    fn zip_links_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        zip.add_symlink(
            "typerr-1.2.0-windows-x86_64/typerr.exe",
            "outside",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.finish().unwrap();
        assert!(
            extract(
                &path,
                &temp.path().join("output"),
                "1.2.0",
                Distribution::Windows
            )
            .is_err()
        );
    }
}
