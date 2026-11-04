//! Portable replacement journal, process handoff, and pacman ownership boundary.
use super::{
    Result,
    archive::{self, hidden_command, output_timeout},
    release::Distribution,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, Stdio},
    time::{Duration, Instant},
};

pub const WORK: &str = ".typerr-update";
pub struct Lease {
    file: File,
    pub root: PathBuf,
}
fn state_dir() -> Result<PathBuf> {
    Ok(crate::storage::database_path()?
        .parent()
        .ok_or("Missing data directory")?
        .join("updates"))
}
fn lock(root: &Path, suffix: &str) -> Result<File> {
    let state = state_dir()?;
    fs::create_dir_all(&state)?;
    let name = format!(
        "{:x}.{suffix}",
        Sha256::digest(root.as_os_str().as_encoded_bytes())
    );
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(state.join(name))?)
}
impl Lease {
    pub fn acquire(root: PathBuf) -> Result<Self> {
        let guard = lock(&root, "installer")?;
        guard
            .try_lock_shared()
            .map_err(|_| "An update is installing. Wait for Typerr to restart.")?;
        let file = lock(&root, "running")?;
        file.try_lock_shared()
            .map_err(|_| "An update is in progress. Wait for it to finish.")?;
        Ok(Self { file, root })
    }
    pub fn exclusive(&self) -> Result<()> {
        // Serialize lock conversion: a second updater cannot win between our
        // shared unlock and reacquisition, and new launches cannot enter.
        let intent = lock(&self.root, "installer")?;
        intent
            .try_lock()
            .map_err(|_| "Another update is starting. Try again shortly.")?;
        self.file.unlock()?;
        if self.file.try_lock().is_err() {
            let _ = self.file.try_lock_shared();
            return Err("Close other Typerr windows before upgrading, then Retry.".into());
        }
        Ok(())
    }
    pub fn shared(&self) {
        let _ = self.file.unlock();
        let _ = self.file.try_lock_shared();
    }
}
pub fn detect(
    root: &Path,
    mut run: impl FnMut(&mut std::process::Command) -> Result<std::process::Output>,
) -> Result<Distribution> {
    if cfg!(windows) {
        return Ok(Distribution::Windows);
    }
    let binary = root.join(archive::executable());
    if Path::new("/usr/bin/pacman").exists() {
        let result = run(hidden_command("/usr/bin/pacman")
            .args(["-Qqo"])
            .arg(&binary))?;
        if result.status.success() {
            if String::from_utf8_lossy(&result.stdout).trim() == "typerr" {
                return Ok(Distribution::Pacman);
            }
            return Err(
                "This executable belongs to another package; update it with its package manager."
                    .into(),
            );
        }
        // Exit 1 is pacman's 'no owner' result. Other errors must not enable self replacement.
        if result.status.code() != Some(1) {
            return Err("Could not determine package ownership".into());
        }
    }
    if root.starts_with("/usr") || root.starts_with("/opt") {
        return Err(
            "System installation has no supported package ownership. Install the release manually."
                .into(),
        );
    }
    Ok(Distribution::Linux)
}
pub fn distribution(root: &Path) -> Result<Distribution> {
    detect(root, |c| output_timeout(c, Duration::from_secs(10)))
}
fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    let temp = path.with_extension("tmp");
    let mut file = File::create_new(&temp)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    fs::rename(temp, path)?;
    sync_dir(path.parent().ok_or("Missing journal directory")?)
}
pub fn read_journal(work: &Path) -> Result<Value> {
    let bytes = fs::read(work.join("journal.json"))?;
    if bytes.len() > 128 * 1024 {
        return Err("Update journal is too large".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn entries(journal: &Value) -> Result<Vec<(String, bool)>> {
    let list = journal["entries"]
        .as_array()
        .ok_or("Missing journal entries")?;
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in list {
        let name = e["name"].as_str().ok_or("Invalid journal entry")?;
        if !archive::allowed(Path::new(name))
            || Path::new(name).components().count() != 1
            || !seen.insert(name)
        {
            return Err("Unsafe journal entry".into());
        }
        result.push((
            name.into(),
            e["existed"].as_bool().ok_or("Invalid journal entry")?,
        ));
    }
    Ok(result)
}
fn remove(path: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err("Refusing an update path containing a symbolic link".into());
        }
        if meta.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
fn no_link(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Installation contains a symbolic link; update manually.".into());
    }
    Ok(())
}
pub fn journal(root: &Path, work: &Path, version: &str, pacman: bool) -> Result<String> {
    let token = format!("{:032x}", rand::random::<u128>());
    let payload = work.join("payload");
    let mut list = Vec::new();
    if !pacman {
        for entry in fs::read_dir(&payload)? {
            let path = entry?.path();
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("Invalid payload name")?;
            if name == "settings.json" && root.join(name).exists() {
                continue;
            }
            no_link(&root.join(name))?;
            list.push(json!({"name":name,"existed":root.join(name).exists()}));
        }
        list.sort_by_key(|e| e["name"].as_str().unwrap_or_default().to_owned());
    }
    write_json(
        &work.join("journal.json"),
        &json!({"root":root,"cwd":std::env::current_dir()?,"version":version,"token":token,"pacman":pacman,"entries":list}),
    )?;
    Ok(token)
}
fn rename_wait(from: &Path, to: &Path) -> Result<()> {
    let start = Instant::now();
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && error.kind() == std::io::ErrorKind::PermissionDenied
                    && start.elapsed() < Duration::from_secs(15) =>
            {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(error) => return Err(error.into()),
        }
    }
}
pub fn apply(root: &Path, work: &Path, journal: &Value) -> Result<()> {
    let backup = work.join("backup");
    fs::create_dir_all(&backup)?;
    for (name, existed) in entries(journal)? {
        let destination = root.join(&name);
        no_link(&destination)?;
        if existed {
            if destination.is_file() {
                let staged_backup = backup.join(format!(".{name}.partial"));
                let mut file = File::create_new(&staged_backup)?;
                std::io::copy(&mut File::open(&destination)?, &mut file)?;
                file.sync_all()?;
                fs::set_permissions(&staged_backup, fs::metadata(&destination)?.permissions())?;
                fs::rename(staged_backup, backup.join(&name))?;
            } else {
                rename_wait(&destination, &backup.join(&name))?;
            }
            sync_dir(&backup)?;
            sync_dir(root)?;
        }
        rename_wait(&work.join("payload").join(&name), &destination)?;
        sync_dir(root)?;
    }
    File::create_new(work.join("applied"))?.sync_all()?;
    sync_dir(work)
}
pub fn rollback(root: &Path, work: &Path, journal: &Value) -> Result<()> {
    for (name, existed) in entries(journal)?.into_iter().rev() {
        let backup = work.join("backup").join(&name);
        let destination = root.join(&name);
        if backup.exists() {
            no_link(&destination)?;
            if backup.is_dir() || destination.is_dir() {
                remove(&destination)?;
            }
            rename_wait(&backup, &destination)?;
        } else if !existed {
            remove(&destination)?;
        }
    }
    sync_dir(root)
}
pub fn prepare_work(root: &Path, kind: Distribution) -> Result<PathBuf> {
    if kind == Distribution::Pacman {
        let state = state_dir()?;
        fs::create_dir_all(&state)?;
        return Ok(tempfile::Builder::new()
            .prefix("package-")
            .tempdir_in(state)?
            .keep());
    }
    let work = root.join(WORK);
    if work.join("confirmed").exists() || work.join("recovered").exists() {
        cleanup(&work);
    }
    if work.exists() {
        return Err("A previous update needs recovery. Restart Typerr before trying again.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(&work)?;
    }
    #[cfg(not(unix))]
    fs::create_dir(&work)?;
    Ok(work)
}
pub fn cleanup(work: &Path) {
    if let Err(e) = remove(work) {
        super::diagnostic(&format!(
            "Could not clean update staging {}: {e}",
            work.display()
        ));
    }
}

pub fn pacman_command(package: &Path) -> std::process::Command {
    let mut command = hidden_command("/usr/bin/pkexec");
    command
        .args([
            "--disable-internal-agent",
            "/usr/bin/pacman",
            "-U",
            "--noconfirm",
        ])
        .arg(package);
    command
}
pub fn install_package(
    package: &Path,
    version: &str,
    mut run: impl FnMut(&mut std::process::Command) -> Result<std::process::Output>,
) -> Result<()> {
    let info = run(hidden_command("/usr/bin/pacman")
        .env("LC_ALL", "C")
        .args(["-Qip"])
        .arg(package))?;
    let text = String::from_utf8_lossy(&info.stdout);
    let field = |key: &str| {
        text.lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.trim() == key)
            .map(|(_, value)| value.trim())
    };
    if !info.status.success()
        || field("Name") != Some("typerr")
        || field("Version") != Some(format!("{version}-1").as_str())
        || field("Architecture") != Some("x86_64")
    {
        return Err("Package name, version, or architecture does not match the release".into());
    }
    if !Path::new("/usr/bin/pkexec").exists() {
        return Err(format!(
            "System authentication is unavailable. Install manually: sudo pacman -U '{}'",
            package.display()
        )
        .into());
    }
    let result = run(&mut pacman_command(package))?;
    if !result.status.success() {
        return Err(format!(
            "Package installation failed or authentication was cancelled. {}\nPackage: {}",
            String::from_utf8_lossy(&result.stderr).trim(),
            package.display()
        )
        .into());
    }
    Ok(())
}
pub fn copy_helper(work: &Path) -> Result<PathBuf> {
    let helper = work.join(if cfg!(windows) {
        "helper.exe"
    } else {
        "helper"
    });
    fs::copy(std::env::current_exe()?, &helper)?;
    OpenOptions::new().write(true).open(&helper)?.sync_all()?;
    Ok(helper)
}
pub fn launch_helper(root: &Path, work: &Path, token: &str) -> Result<ChildStdin> {
    let helper = work.join(if cfg!(windows) {
        "helper.exe"
    } else {
        "helper"
    });
    let mut child = hidden_command(&helper)
        .arg("--typerr-update-helper")
        .arg(root)
        .arg(work)
        .arg(token)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let pipe = child.stdin.take().ok_or("Could not start update helper")?;
    let start = Instant::now();
    while !work.join("ready").exists() {
        if child.try_wait()?.is_some() || start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Update helper could not start. Installed files have not been changed.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    // The helper is reaped by the OS after this process exits.
    Ok(pipe)
}
fn field_path(journal: &Value, name: &str) -> Result<PathBuf> {
    let p = PathBuf::from(journal[name].as_str().ok_or("Invalid journal path")?);
    if !p.is_absolute() {
        return Err("Relative update path".into());
    }
    Ok(p)
}
fn helper(root: &Path, work: &Path, token: &str) -> Result<()> {
    let journal = read_journal(work)?;
    if journal["token"] != token || field_path(&journal, "root")? != root {
        return Err("Invalid helper handoff".into());
    }
    let installer = lock(root, "installer")?;
    installer.try_lock()?;
    File::create_new(work.join("ready"))?.sync_all()?;
    // Parent closes this pipe only after draining scores and releasing its run lease.
    std::io::stdin().read_to_end(&mut Vec::new())?;
    let running = lock(root, "running")?;
    let start = Instant::now();
    while running.try_lock().is_err() {
        if start.elapsed() > Duration::from_secs(15) {
            return Err(
                "Another Typerr process prevented installation; restart to recover.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let pacman = journal["pacman"] == true;
    if !pacman && let Err(error) = apply(root, work, &journal) {
        rollback(root, work, &journal)?;
        fs::write(work.join("failure"), error.to_string())?;
        running.unlock()?;
        drop(installer);
        hidden_command(root.join(archive::executable()))
            .current_dir(field_path(&journal, "cwd")?)
            .spawn()?;
        return Err(error);
    }
    running.unlock()?;
    let launched = hidden_command(root.join(archive::executable()))
        .arg("--typerr-update-confirm")
        .arg(work)
        .arg(token)
        .current_dir(field_path(&journal, "cwd")?)
        .spawn();
    let mut child = match launched {
        Ok(child) => child,
        Err(error) => {
            running.try_lock()?;
            if !pacman {
                rollback(root, work, &journal)?;
                fs::write(
                    work.join("failure"),
                    format!(
                        "The updated app could not start: {error}. The previous version was restored."
                    ),
                )?;
            }
            running.unlock()?;
            drop(installer);
            hidden_command(root.join(archive::executable()))
                .current_dir(field_path(&journal, "cwd")?)
                .spawn()?;
            return Err(error.into());
        }
    };
    let start = Instant::now();
    while !work.join("confirmed").exists() {
        if child.try_wait()?.is_some() || start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            running.try_lock()?;
            if !pacman {
                rollback(root, work, &journal)?;
                fs::write(
                    work.join("failure"),
                    "The new version did not finish startup. The previous version was restored.",
                )?;
            }
            running.unlock()?;
            drop(installer);
            hidden_command(root.join(archive::executable()))
                .current_dir(field_path(&journal, "cwd")?)
                .spawn()?;
            return Err("Updated application failed to confirm startup".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    sync_dir(work)?;
    // Windows cannot delete this running helper; the next launch cleans its journal.
    if !cfg!(windows) {
        cleanup(work);
    }
    Ok(())
}
fn recover_helper(root: &Path, work: &Path) -> Result<()> {
    let journal = read_journal(work)?;
    if field_path(&journal, "root")? != root {
        return Err("Invalid recovery root".into());
    }
    let installer = lock(root, "installer")?;
    installer.lock()?;
    let running = lock(root, "running")?;
    running.lock()?;
    // On Windows the launching process may still hold its executable briefly.
    let start = Instant::now();
    loop {
        match rollback(root, work, &journal) {
            Ok(()) => break,
            Err(_) if cfg!(windows) && start.elapsed() < Duration::from_secs(15) => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(error) => return Err(error),
        }
    }
    File::create(work.join("recovered"))?.sync_all()?;
    running.unlock()?;
    drop(installer);
    hidden_command(root.join(archive::executable()))
        .current_dir(field_path(&journal, "cwd")?)
        .spawn()?;
    Ok(())
}
pub struct Startup {
    pub lease: Option<Lease>,
    pub confirmation: Option<PathBuf>,
    pub error: Option<String>,
}
pub enum Boot {
    Exit,
    Run(Startup),
}
pub fn bootstrap() -> Result<Boot> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--version") {
        println!("Typerr {}", env!("CARGO_PKG_VERSION"));
        return Ok(Boot::Exit);
    }
    if args.first().is_some_and(|a| a == "--typerr-update-helper") {
        if args.len() != 4 {
            return Err("Invalid update helper arguments".into());
        }
        helper(
            Path::new(&args[1]),
            Path::new(&args[2]),
            args[3].to_str().ok_or("Invalid token")?,
        )?;
        return Ok(Boot::Exit);
    }
    if args.first().is_some_and(|a| a == "--typerr-recover") {
        if args.len() != 3 {
            return Err("Invalid recovery arguments".into());
        }
        recover_helper(Path::new(&args[1]), Path::new(&args[2]))?;
        return Ok(Boot::Exit);
    }
    let root = std::env::current_exe()?
        .canonicalize()?
        .parent()
        .ok_or("Missing installation directory")?
        .to_owned();
    if args.first().is_some_and(|a| a == "--typerr-update-confirm") {
        if args.len() != 3 {
            return Err("Invalid confirmation arguments".into());
        }
        let work = PathBuf::from(&args[1]);
        let journal = read_journal(&work)?;
        if field_path(&journal, "root")? != root
            || journal["token"].as_str() != args[2].to_str()
            || journal["version"] != env!("CARGO_PKG_VERSION")
        {
            return Err("Update confirmation does not match this executable".into());
        }
        let file = lock(&root, "running")?;
        file.try_lock_shared()?;
        return Ok(Boot::Run(Startup {
            lease: Some(Lease { file, root }),
            confirmation: Some(work),
            error: None,
        }));
    }
    if !args.is_empty() {
        return Err("Unknown Typerr command line option".into());
    }
    let installer = match lock(&root, "installer") {
        Ok(file) => file,
        Err(error) if !root.join(WORK).exists() => {
            super::diagnostic(&format!("Updater disabled: {error}"));
            return Ok(Boot::Run(Startup {
                lease: None,
                confirmation: None,
                error: None,
            }));
        }
        Err(error) => return Err(error),
    };
    installer
        .try_lock()
        .map_err(|_| "An update is installing. Wait for Typerr to restart.")?;
    let work = root.join(WORK);
    no_link(&work)?;
    let mut error = None;
    if work.exists() {
        let running = lock(&root, "running")?;
        running
            .try_lock()
            .map_err(|_| "An update is active in another Typerr window.")?;
        if work.join("journal.json").exists() {
            let journal = read_journal(&work)?;
            if field_path(&journal, "root")? != root {
                return Err("Invalid update recovery path".into());
            }
            if !work.join("confirmed").exists() {
                if cfg!(windows) && work.join("backup").exists() && !work.join("recovered").exists()
                {
                    let recovery = work.join("recovery.exe");
                    fs::copy(std::env::current_exe()?, &recovery)?;
                    hidden_command(&recovery)
                        .arg("--typerr-recover")
                        .arg(&root)
                        .arg(&work)
                        .spawn()?;
                    return Ok(Boot::Exit);
                }
                if !work.join("recovered").exists() {
                    rollback(&root, &work, &journal)?;
                }
                error = Some(
                    fs::read_to_string(work.join("failure")).unwrap_or_else(|_| {
                        "An interrupted update was rolled back. You can try upgrading again.".into()
                    }),
                );
            }
        }
        cleanup(&work);
    }
    drop(installer);
    Ok(Boot::Run(Startup {
        lease: Some(Lease::acquire(root)?),
        confirmation: None,
        error,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, PathBuf, Value) {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join(WORK);
        fs::create_dir_all(work.join("payload")).unwrap();
        fs::write(dir.path().join("typerr"), "old").unwrap();
        fs::write(dir.path().join("settings.json"), "my settings").unwrap();
        fs::write(work.join("payload/typerr"), "new").unwrap();
        fs::write(work.join("payload/README.md"), "new readme").unwrap();
        fs::write(work.join("payload/settings.json"), "defaults").unwrap();
        journal(dir.path(), &work, "1.2.0", false).unwrap();
        let j = read_journal(&work).unwrap();
        (dir, work, j)
    }
    #[test]
    fn replacement_and_rollback_preserve_settings() {
        let (dir, work, j) = setup();
        apply(dir.path(), &work, &j).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("typerr")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            "my settings"
        );
        rollback(dir.path(), &work, &j).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("typerr")).unwrap(),
            "old"
        );
        assert!(!dir.path().join("README.md").exists());
    }
    #[test]
    fn interrupted_rename_is_recoverable() {
        let (dir, work, j) = setup();
        fs::create_dir(work.join("backup")).unwrap();
        fs::rename(dir.path().join("typerr"), work.join("backup/typerr")).unwrap();
        rollback(dir.path(), &work, &j).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("typerr")).unwrap(),
            "old"
        );
        rollback(dir.path(), &work, &j).unwrap();
    }
    #[test]
    fn failures_restore_old_files() {
        let (dir, work, j) = setup();
        fs::remove_file(work.join("payload/typerr")).unwrap();
        assert!(apply(dir.path(), &work, &j).is_err());
        rollback(dir.path(), &work, &j).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("typerr")).unwrap(),
            "old"
        );
    }
    #[test]
    fn package_command_does_not_use_a_shell() {
        let c = pacman_command(Path::new("/tmp/a b.pkg.tar.zst"));
        assert_eq!(c.get_program(), "/usr/bin/pkexec");
        assert_eq!(c.get_args().last().unwrap(), "/tmp/a b.pkg.tar.zst");
        assert!(!c.get_args().any(|s| s == "--overwrite"));
    }
    #[test]
    fn corrupted_journal_cannot_escape() {
        assert!(entries(&json!({"entries":[{"name":"../outside","existed":false}]})).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn package_metadata_and_authentication_are_checked() {
        use std::os::unix::process::ExitStatusExt;
        let output = |code, text: &str| std::process::Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: text.as_bytes().to_vec(),
            stderr: Vec::new(),
        };
        let mut calls = 0;
        assert!(
            install_package(Path::new("/tmp/test.pkg.tar.zst"), "1.2.0", |_| {
                calls += 1;
                Ok(output(
                    0,
                    "Name : wrong\nVersion : 1.2.0-1\nArchitecture : x86_64\n",
                ))
            })
            .is_err()
        );
        assert_eq!(calls, 1);
        if Path::new("/usr/bin/pkexec").exists() {
            let mut calls = 0;
            assert!(
                install_package(Path::new("/tmp/test.pkg.tar.zst"), "1.2.0", |command| {
                    calls += 1;
                    if calls == 1 {
                        Ok(output(
                            0,
                            "Name : typerr\nVersion : 1.2.0-1\nArchitecture : x86_64\n",
                        ))
                    } else {
                        assert_eq!(command.get_program(), "/usr/bin/pkexec");
                        Ok(output(126, ""))
                    }
                })
                .is_err()
            );
            assert_eq!(calls, 2);
        }
    }
    #[cfg(unix)]
    #[test]
    fn package_ownership_is_never_overwritten() {
        use std::os::unix::process::ExitStatusExt;
        if !Path::new("/usr/bin/pacman").exists() {
            return;
        }
        for (name, expected) in [("typerr", true), ("some-other-package", false)] {
            let result = detect(Path::new("/usr/lib/typerr"), |command| {
                assert_eq!(command.get_args().next().unwrap(), "-Qqo");
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: name.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
            assert_eq!(result.is_ok(), expected);
            if expected {
                assert_eq!(result.unwrap(), Distribution::Pacman);
            }
        }
    }
    #[test]
    fn operating_system_locks_detect_other_instances() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lock");
        let first = File::create(&path).unwrap();
        let second = File::open(&path).unwrap();
        first.try_lock_shared().unwrap();
        second.try_lock_shared().unwrap();
        first.unlock().unwrap();
        assert!(first.try_lock().is_err());
        drop(second);
        first.try_lock().unwrap();
    }
}
