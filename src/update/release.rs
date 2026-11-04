//! Strict release selection and bounded HTTPS downloads. No application state.
use super::{Error, Result};
use semver::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub const REPOSITORY: &str = "https://github.com/Arik-Ahnaf/Typerr";
pub const API: &str = "https://api.github.com/repos/Arik-Ahnaf/Typerr/releases/latest";
pub const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    Windows,
    Linux,
    Pacman,
}
#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub digest: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Release {
    pub version: Version,
    pub archive: Asset,
    pub checksums: Asset,
}

fn invalid(message: &str) -> Error {
    message.into()
}
fn asset(value: &Value, tag: &str, name: &str, limit: u64) -> Result<Asset> {
    let size = value["size"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= limit)
        .ok_or_else(|| invalid("Invalid release asset size"))?;
    let url = value["browser_download_url"]
        .as_str()
        .ok_or_else(|| invalid("Missing asset URL"))?;
    if url != format!("{REPOSITORY}/releases/download/{tag}/{name}") || value["state"] != "uploaded"
    {
        return Err(invalid("Unexpected release asset URL or upload state"));
    }
    let digest = match value.get("digest").filter(|v| !v.is_null()) {
        Some(v) => Some(
            v.as_str()
                .and_then(|s| s.strip_prefix("sha256:"))
                .filter(|s| valid_hash(s))
                .ok_or_else(|| invalid("Invalid GitHub asset digest"))?
                .to_ascii_lowercase(),
        ),
        None => None,
    };
    Ok(Asset {
        name: name.into(),
        url: url.into(),
        size,
        digest,
    })
}
pub fn parse(bytes: &[u8], current: &str, distribution: Distribution) -> Result<Option<Release>> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        return Ok(None);
    }
    let tag = value["tag_name"]
        .as_str()
        .ok_or_else(|| invalid("Missing release tag"))?;
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag))?;
    if !version.pre.is_empty() || !version.cmp_precedence(&Version::parse(current)?).is_gt() {
        return Ok(None);
    }
    let name = match distribution {
        Distribution::Windows => format!("typerr-{version}-windows-x86_64.zip"),
        Distribution::Linux => format!("typerr-{version}-arch-linux-x86_64.tar.gz"),
        Distribution::Pacman => format!("typerr-{version}-1-x86_64.pkg.tar.zst"),
    };
    let assets = value["assets"]
        .as_array()
        .ok_or_else(|| invalid("Missing release assets"))?;
    let unique = |name: &str, limit| -> Result<Asset> {
        let mut matches = assets.iter().filter(|a| a["name"] == name);
        let found = matches
            .next()
            .ok_or_else(|| invalid(&format!("Release is missing {name}")))?;
        if matches.next().is_some() {
            return Err(invalid("Duplicate release asset"));
        }
        asset(found, tag, name, limit)
    };
    Ok(Some(Release {
        version,
        archive: unique(&name, MAX_ARCHIVE)?,
        checksums: unique("SHA256SUMS", 64 * 1024)?,
    }))
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn checksum(bytes: &[u8], name: &str) -> Result<String> {
    let text = std::str::from_utf8(bytes)?;
    let mut found = None;
    for line in text.lines() {
        let Some((hash, file)) = line.split_once("  ") else {
            return Err(invalid("Invalid SHA256SUMS entry"));
        };
        if !valid_hash(hash) {
            return Err(invalid("Invalid SHA256SUMS hash"));
        }
        if file == name && found.replace(hash.to_ascii_lowercase()).is_some() {
            return Err(invalid("Duplicate checksum entry"));
        }
    }
    found.ok_or_else(|| invalid("Archive is missing from SHA256SUMS"))
}
fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .build()
        .into()
}
fn allowed_url(url: &str) -> bool {
    // Parse with the client's HTTP URI parser, never with string suffix matching.
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && matches!(
            uri.host(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
        && uri.port_u16().is_none_or(|p| p == 443)
        && !uri.authority().is_some_and(|a| a.as_str().contains('@'))
}
fn request(url: &str, timeout: Duration) -> Result<ureq::http::Response<ureq::Body>> {
    let agent = agent(timeout);
    let start = std::time::Instant::now();
    let mut url = url.to_owned();
    for _ in 0..=5 {
        if !allowed_url(&url) {
            return Err(invalid("Update URL is not an allowed GitHub HTTPS address"));
        }
        let remaining = timeout
            .checked_sub(start.elapsed())
            .ok_or_else(|| invalid("Update request timed out"))?;
        let response = agent
            .get(&url)
            .header("User-Agent", concat!("Typerr/", env!("CARGO_PKG_VERSION")))
            .header("Accept", "application/vnd.github+json")
            .config()
            .timeout_global(Some(remaining))
            .build()
            .call()?;
        if response.status().is_redirection() {
            url = response
                .headers()
                .get("location")
                .ok_or_else(|| invalid("Missing redirect location"))?
                .to_str()?
                .to_owned();
            continue;
        }
        if response.status() != 200 {
            return Err(invalid(&format!("GitHub returned {}", response.status())));
        }
        return Ok(response);
    }
    Err(invalid("Too many update redirects"))
}
pub fn check(distribution: Distribution) -> Result<Option<Release>> {
    let mut response = request(API, Duration::from_secs(10))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("Release metadata is too large"));
    }
    parse(&bytes, env!("CARGO_PKG_VERSION"), distribution)
}
pub fn download(
    asset: &Asset,
    target: &Path,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> Result<String> {
    let mut response = request(&asset.url, Duration::from_secs(600))?;
    save_download(
        asset,
        target,
        cancel,
        progress,
        &mut response.body_mut().as_reader(),
    )
}
fn save_download(
    asset: &Asset,
    target: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
    reader: &mut dyn Read,
) -> Result<String> {
    let mut file = File::create_new(target)?;
    let mut hash = Sha256::new();
    let mut total = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(invalid("Update cancelled"));
        }
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > asset.size || total > MAX_ARCHIVE {
            return Err(invalid("Download exceeds its declared size"));
        }
        file.write_all(&buffer[..n])?;
        hash.update(&buffer[..n]);
        progress(total, asset.size);
    }
    if total != asset.size {
        return Err(invalid("Incomplete download"));
    }
    file.sync_all()?;
    let hash = format!("{:x}", hash.finalize());
    if asset
        .digest
        .as_ref()
        .is_some_and(|expected| expected != &hash)
    {
        return Err(invalid("GitHub asset checksum mismatch"));
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(version: &str) -> Value {
        let tag = format!("v{version}");
        let assets: Vec<_> = [format!("typerr-{version}-windows-x86_64.zip"), "SHA256SUMS".into()].into_iter().map(|name| serde_json::json!({"size":42,"state":"uploaded","browser_download_url":format!("{REPOSITORY}/releases/download/{tag}/{name}"),"name":name})).collect();
        serde_json::json!({"tag_name":tag,"draft":false,"prerelease":false,"assets":assets})
    }
    #[test]
    fn versions_and_channels() {
        for (remote, expected) in [
            ("1.0.0", false),
            ("1.1.0", false),
            ("1.1.0+build.2", false),
            ("1.2.0", true),
            ("1.10.0", true),
            ("2.0.0-beta.1", false),
        ] {
            assert_eq!(
                parse(
                    &serde_json::to_vec(&fixture(remote)).unwrap(),
                    "1.1.0",
                    Distribution::Windows
                )
                .unwrap()
                .is_some(),
                expected
            );
        }
        for field in ["draft", "prerelease"] {
            let mut f = fixture("1.2.0");
            f[field] = true.into();
            assert!(
                parse(
                    &serde_json::to_vec(&f).unwrap(),
                    "1.1.0",
                    Distribution::Windows
                )
                .unwrap()
                .is_none()
            );
        }
    }
    #[test]
    fn reject_bad_metadata() {
        for f in [
            serde_json::json!({"draft":false,"prerelease":false,"tag_name":"oops"}),
            serde_json::json!({"draft":false,"prerelease":false,"tag_name":"v2.0.0","assets":[]}),
        ] {
            assert!(
                parse(
                    &serde_json::to_vec(&f).unwrap(),
                    "1.1.0",
                    Distribution::Windows
                )
                .is_err()
            );
        }
        assert!(parse(b"broken", "1.1.0", Distribution::Windows).is_err());
        let mut f = fixture("1.2.0");
        f["assets"][0]["browser_download_url"] = "https://evil.example/file".into();
        assert!(
            parse(
                &serde_json::to_vec(&f).unwrap(),
                "1.1.0",
                Distribution::Windows
            )
            .is_err()
        );
    }
    #[test]
    fn checksums_and_redirects() {
        let line = format!("{}  test.zip\n", "a".repeat(64));
        assert_eq!(
            checksum(line.as_bytes(), "test.zip").unwrap(),
            "a".repeat(64)
        );
        assert!(checksum(line.repeat(2).as_bytes(), "test.zip").is_err());
        assert!(checksum(line.as_bytes(), "other.zip").is_err());
        for url in [
            "http://github.com/a",
            "https://github.com.evil/a",
            "https://user@github.com/a",
            "file:///tmp/a",
        ] {
            assert!(!allowed_url(url));
        }
        assert!(allowed_url(
            "https://release-assets.githubusercontent.com/a?token=x"
        ));
    }
    #[test]
    fn streamed_downloads_reject_corruption_truncation_excess_and_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        let asset = Asset {
            name: "test".into(),
            url: String::new(),
            size: 3,
            digest: Some(format!("{:x}", Sha256::digest(b"abc"))),
        };
        let cancel = AtomicBool::new(false);
        for (i, bytes) in [b"abc".as_slice(), b"bad", b"ab", b"abcd"]
            .into_iter()
            .enumerate()
        {
            let result = save_download(
                &asset,
                &dir.path().join(i.to_string()),
                &cancel,
                |_, _| {},
                &mut &bytes[..],
            );
            assert_eq!(result.is_ok(), i == 0);
        }
        cancel.store(true, Ordering::Relaxed);
        assert!(
            save_download(
                &asset,
                &dir.path().join("cancel"),
                &cancel,
                |_, _| {},
                &mut &b"abc"[..]
            )
            .is_err()
        );
    }
}
