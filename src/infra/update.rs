//! Self-update: check the latest GitHub release and install the verified binary.

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use crate::error::{AnirelError, Result};

const LATEST_URL: &str = "https://api.github.com/repos/Praveensenpai/anirel/releases/latest";
const RELEASES_URL: &str = "https://github.com/Praveensenpai/anirel/releases/latest";
const ARCHIVE_NAME: &str = "anirel-x86_64-linux.tar.gz";
const CHECKSUM_NAME: &str = "anirel-x86_64-linux.tar.gz.sha256";
const BINARY_NAME: &str = "anirel";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// What the version comparison decided to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    UpToDate,
    Upgrade,
    NewerInstalled,
}

/// Outcome of an update attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateReport {
    pub current: String,
    pub latest: String,
    pub updated: bool,
}

/// Compare dotted versions; missing components count as zero.
pub fn decide(current: &str, latest: &str) -> Decision {
    let current = components(current);
    let latest = components(latest);
    for index in 0..current.len().max(latest.len()) {
        let installed = current.get(index).copied().unwrap_or(0);
        let released = latest.get(index).copied().unwrap_or(0);
        if installed < released {
            return Decision::Upgrade;
        }
        if installed > released {
            return Decision::NewerInstalled;
        }
    }
    Decision::UpToDate
}

fn components(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

/// Check for a newer release and install it only when versions differ.
pub async fn run() -> Result<UpdateReport> {
    let client = reqwest::Client::new();
    let release = fetch_release(&client).await?;
    let latest = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    let current = env!("CARGO_PKG_VERSION").to_string();

    let mut report = UpdateReport {
        current,
        latest,
        updated: false,
    };
    if decide(&report.current, &report.latest) != Decision::Upgrade {
        return Ok(report);
    }

    let archive = download(&client, asset_url(&release, ARCHIVE_NAME)?).await?;
    let checksum = download(&client, asset_url(&release, CHECKSUM_NAME)?).await?;
    let expected = expected_hash(&String::from_utf8_lossy(&checksum))?;
    let actual = sha256_hex(&archive);
    if expected != actual {
        return Err(AnirelError::Update(format!(
            "checksum mismatch for {ARCHIVE_NAME}: expected {expected}, got {actual}"
        )));
    }
    let binary = extract_binary(&archive)?;
    install(&binary, &std::env::current_exe()?)?;
    report.updated = true;
    Ok(report)
}

async fn fetch_release(client: &reqwest::Client) -> Result<Release> {
    client
        .get(LATEST_URL)
        .header(USER_AGENT, format!("anirel/{}", env!("CARGO_PKG_VERSION")))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await
        .map_err(Into::into)
}

fn asset_url<'a>(release: &'a Release, name: &str) -> Result<&'a str> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.as_str())
        .ok_or_else(|| {
            AnirelError::Update(format!(
                "release {} has no {name} asset; see {RELEASES_URL}",
                release.tag_name
            ))
        })
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .header(USER_AGENT, format!("anirel/{}", env!("CARGO_PKG_VERSION")))
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

fn expected_hash(text: &str) -> Result<String> {
    text.split_whitespace()
        .next()
        .map(|hash| hash.to_ascii_lowercase())
        .ok_or_else(|| AnirelError::Update("the release checksum file is empty".into()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn extract_binary(archive: &[u8]) -> Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(archive);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let is_binary =
            entry.path()?.file_name().and_then(|name| name.to_str()) == Some(BINARY_NAME);
        if is_binary {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            return Ok(bytes);
        }
    }
    Err(AnirelError::Update(format!(
        "{ARCHIVE_NAME} contains no {BINARY_NAME} binary"
    )))
}

/// A temp binary that is removed unless the rename succeeds.
struct TempBinary(PathBuf);

impl Drop for TempBinary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn install(binary: &[u8], target: &Path) -> Result<()> {
    let directory = target.parent().ok_or_else(|| {
        AnirelError::Update(format!("no parent directory for {}", target.display()))
    })?;
    let temp = TempBinary(directory.join(format!(".anirel-update-{}", std::process::id())));
    std::fs::write(&temp.0, binary)?;
    make_executable(&temp.0)?;
    std::fs::rename(&temp.0, target).map_err(|error| {
        AnirelError::Update(format!(
            "could not replace {}: {error}; install manually from {RELEASES_URL}",
            target.display()
        ))
    })?;
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Decision, decide, expected_hash, sha256_hex};

    #[test]
    fn equal_versions_are_left_alone() {
        assert_eq!(decide("0.1.3", "v0.1.3"), Decision::UpToDate);
        assert_eq!(decide("0.1.3", "0.1.3.0"), Decision::UpToDate);
    }

    #[test]
    fn a_newer_release_triggers_an_upgrade() {
        assert_eq!(decide("0.1.3", "v0.2.0"), Decision::Upgrade);
        assert_eq!(decide("0.1.3", "v0.1.4"), Decision::Upgrade);
    }

    #[test]
    fn a_newer_installed_version_is_never_downgraded() {
        assert_eq!(decide("0.2.0", "v0.1.3"), Decision::NewerInstalled);
    }

    #[test]
    fn prerelease_suffixes_do_not_hide_the_number() {
        assert_eq!(decide("0.1.3", "v0.1.4-beta"), Decision::Upgrade);
    }

    #[test]
    fn checksums_are_case_insensitive_hex() {
        let hash = sha256_hex(b"anirel");
        let line = format!("{}  anirel-x86_64-linux.tar.gz\n", hash.to_uppercase());
        assert_eq!(expected_hash(&line).expect("a hash"), hash);
    }

    #[test]
    fn an_empty_checksum_is_rejected() {
        assert!(expected_hash("  \n").is_err());
    }

    #[test]
    fn an_archive_yields_only_the_binary() {
        use flate2::{Compression, write::GzEncoder};
        use tar::{Builder, Header};

        let mut encoded = Vec::new();
        {
            let encoder = GzEncoder::new(&mut encoded, Compression::default());
            let mut builder = Builder::new(encoder);

            let mut header = Header::new_gnu();
            header.set_size(5);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "README.md", b"notes".as_slice())
                .expect("append readme");

            let mut header = Header::new_gnu();
            header.set_size(7);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "anirel", b"NEW_BIN".as_slice())
                .expect("append binary");

            builder
                .into_inner()
                .expect("into_inner")
                .finish()
                .expect("flush");
        }
        assert_eq!(super::extract_binary(&encoded).expect("binary"), b"NEW_BIN");
    }

    #[test]
    fn install_replaces_the_target_with_executable_permissions() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let path = std::env::temp_dir().join(format!(
            "anirel-update-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, b"OLD").expect("seed target");
        super::install(b"NEW", &path).expect("install");

        assert_eq!(std::fs::read(&path).expect("read"), b"NEW");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o755, "executable permissions");
        }
        std::fs::remove_file(&path).ok();
    }
}
