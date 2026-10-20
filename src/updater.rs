//! Opt-in GitHub Releases updates. Plans retain one immutable tag; payloads must
//! pass both manifest and optional GitHub digest checks before replacement.
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const REPOSITORY: &str = "ersinkoc/rustcalc";
pub const RELEASES_URL: &str = "https://github.com/ersinkoc/rustcalc/releases";
pub const API_URL: &str = "https://api.github.com/repos/ersinkoc/rustcalc/releases/latest";
pub const WINDOWS_ASSET: &str = "rustcalc-windows-x86_64.exe";
pub const MAX_BINARY_SIZE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    #[serde(default)]
    pub digest: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Debug)]
pub struct UpdatePlan {
    pub version: Version,
    pub tag: String,
    pub binary: Asset,
    pub manifest: Asset,
}

/// Kept injectable so tests can exercise the complete validation and staging
/// flow without network access or changing their own test executable.
pub trait Fetch {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String>;
}
pub struct GitHub;
impl Fetch for GitHub {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .https_only(true)
                .timeout_global(Some(Duration::from_secs(90)))
                .timeout_connect(Some(Duration::from_secs(15)))
                .build(),
        );
        let mut response = agent
            .get(url)
            .header(
                "User-Agent",
                concat!("RustCalc/", env!("CARGO_PKG_VERSION")),
            )
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| match e {
                ureq::Error::StatusCode(404) => {
                    "No published stable release is available yet.".to_owned()
                }
                ureq::Error::StatusCode(403 | 429) => {
                    "GitHub rate limit reached. Please try again later.".to_owned()
                }
                _ => format!("Update request failed: {e}"),
            })?;
        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|e| format!("Could not read update response: {e}"))
    }
}

pub fn plan_release(release: Release, current: &str) -> Result<Option<UpdatePlan>, String> {
    let current = Version::parse(current).map_err(|_| "Invalid current version")?;
    let tag = release.tag_name.clone();
    let version = Version::parse(
        tag.strip_prefix('v')
            .ok_or("Release tags must start with v")?,
    )
    .map_err(|_| "Invalid release version")?;
    if release.draft || release.prerelease || !version.pre.is_empty() || !version.build.is_empty() {
        return Err("Only stable releases can be installed".into());
    }
    if version <= current {
        return Ok(None);
    }
    let select = |name: &str, limit: u64| -> Result<Asset, String> {
        let candidates: Vec<_> = release.assets.iter().filter(|a| a.name == name).collect();
        if candidates.len() != 1 {
            return Err(format!("Release must contain exactly one {name}"));
        }
        let asset = candidates[0];
        let expected = format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}");
        if asset.browser_download_url != expected || asset.size == 0 || asset.size > limit {
            return Err(format!("Invalid release asset: {name}"));
        }
        Ok(asset.clone())
    };
    let binary = select(WINDOWS_ASSET, MAX_BINARY_SIZE)?;
    let manifest = select("SHA256SUMS", 1024 * 1024)?;
    Ok(Some(UpdatePlan {
        version,
        tag,
        binary,
        manifest,
    }))
}

pub fn check_with(fetch: &impl Fetch, current: &str) -> Result<Option<UpdatePlan>, String> {
    let bytes = fetch.get(API_URL, 2 * 1024 * 1024)?;
    let release: Release =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid release metadata: {e}"))?;
    plan_release(release, current)
}
pub fn check() -> Result<Option<UpdatePlan>, String> {
    ensure_supported_platform()?;
    check_with(&GitHub, env!("CARGO_PKG_VERSION"))
}
fn ensure_supported_platform() -> Result<(), String> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Ok(())
    } else {
        Err("Automatic updates currently support Windows x64 only. Build from source on this platform.".into())
    }
}

pub fn checksum_for(manifest: &str, name: &str) -> Result<String, String> {
    let mut found = None;
    for line in manifest.lines().filter(|s| !s.trim().is_empty()) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 {
            return Err("Malformed SHA256SUMS entry".into());
        }
        if parts[1].trim_start_matches('*') != name {
            continue;
        }
        if found.is_some() {
            return Err("Duplicate checksum entry".into());
        }
        if parts[0].len() != 64 || !parts[0].bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid SHA-256 checksum".into());
        }
        found = Some(parts[0].to_ascii_lowercase());
    }
    found.ok_or_else(|| format!("Missing checksum for {name}"))
}

pub fn verify_payload(plan: &UpdatePlan, bytes: &[u8], expected: &str) -> Result<(), String> {
    if bytes.len() as u64 != plan.binary.size || bytes.len() as u64 > MAX_BINARY_SIZE {
        return Err("Downloaded file size does not match the release".into());
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(
            "SHA-256 verification failed; the installed application was not changed".into(),
        );
    }
    if let Some(digest) = &plan.binary.digest
        && digest != &format!("sha256:{actual}")
    {
        return Err("GitHub asset digest verification failed".into());
    }
    if !bytes.starts_with(b"MZ") {
        return Err("The release asset is not a Windows executable".into());
    }
    Ok(())
}

pub fn install_with(
    fetch: &impl Fetch,
    plan: &UpdatePlan,
    replace: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    install_using(fetch, plan, verify_candidate_version, replace)
}

fn install_using(
    fetch: &impl Fetch,
    plan: &UpdatePlan,
    verify: impl FnOnce(&Path, &Version) -> Result<(), String>,
    replace: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    // Revalidate public plan fields so callers cannot bypass source/size policy.
    let release = Release {
        tag_name: plan.tag.clone(),
        draft: false,
        prerelease: false,
        assets: vec![plan.binary.clone(), plan.manifest.clone()],
    };
    let validated = plan_release(release, env!("CARGO_PKG_VERSION"))?
        .ok_or("Refusing to install the current or an older version")?;
    if validated.version != plan.version {
        return Err("Update plan version does not match its release tag".into());
    }
    let manifest = fetch.get(&plan.manifest.browser_download_url, 1024 * 1024)?;
    let manifest = std::str::from_utf8(&manifest).map_err(|_| "SHA256SUMS is not UTF-8")?;
    let expected = checksum_for(manifest, &plan.binary.name)?;
    let bytes = fetch.get(&plan.binary.browser_download_url, MAX_BINARY_SIZE)?;
    verify_payload(plan, &bytes, &expected)?;
    let mut staged = tempfile::Builder::new()
        .prefix("rustcalc-update-")
        .suffix(".exe")
        .tempfile()
        .map_err(|e| e.to_string())?;
    staged.write_all(&bytes).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    // Close the file handle before Windows opens it for replacement.
    let staged = staged.into_temp_path();
    verify(&staged, &plan.version)?;
    replace(&staged)
}

/// Check the already hash-verified candidate without launching its GUI. Version
/// output is bounded; a hung or noisy process cannot stall the updater forever.
pub fn verify_candidate_version(path: &Path, expected: &Version) -> Result<(), String> {
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: keep installer/update probes out of the user's desktop.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not run the staged update for version verification: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match result {
                    Err(error) => {
                        format!("Could not read the staged update process status: {error}")
                    }
                    _ => "Update version verification timed out".into(),
                });
            }
        }
    };
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .ok_or("Invalid update version response")?
        .take(129)
        .read_to_end(&mut output)
        .map_err(|_| "Invalid update version response")?;
    let expected = format!("rustcalc {expected}");
    if !status.success()
        || output.len() > 128
        || std::str::from_utf8(&output).map(str::trim) != Ok(expected.as_str())
    {
        return Err("The downloaded executable does not match the release version".into());
    }
    Ok(())
}
pub fn install(plan: &UpdatePlan) -> Result<(), String> {
    ensure_supported_platform()?;
    install_with(&GitHub, plan, |path| {
        self_replace::self_replace(path).map_err(|e| {
            format!("Could not replace the executable; close other RustCalc windows and retry: {e}")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, collections::HashMap};
    fn release(version: &str, bytes: &[u8]) -> Release {
        Release {
            tag_name: format!("v{version}"),
            draft: false,
            prerelease: false,
            assets: vec![
                Asset {
                    name: WINDOWS_ASSET.into(),
                    browser_download_url: format!(
                        "https://github.com/{REPOSITORY}/releases/download/v{version}/{WINDOWS_ASSET}"
                    ),
                    size: bytes.len() as u64,
                    digest: Some(format!("sha256:{:x}", Sha256::digest(bytes))),
                },
                Asset {
                    name: "SHA256SUMS".into(),
                    browser_download_url: format!(
                        "https://github.com/{REPOSITORY}/releases/download/v{version}/SHA256SUMS"
                    ),
                    size: 100,
                    digest: None,
                },
            ],
        }
    }
    struct Fixture(HashMap<String, Vec<u8>>);
    impl Fetch for Fixture {
        fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
            let b = self.0.get(url).ok_or("Unknown fixture URL")?;
            if b.len() as u64 > limit {
                return Err("Response too large".into());
            }
            Ok(b.clone())
        }
    }
    #[test]
    fn stable_semver_comparison_never_downgrades() {
        assert!(
            plan_release(release("0.1.0", b"MZfixture"), "0.1.0")
                .unwrap()
                .is_none()
        );
        assert!(
            plan_release(release("0.0.9", b"MZfixture"), "0.1.0")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            plan_release(release("0.10.0", b"MZfixture"), "0.9.0")
                .unwrap()
                .unwrap()
                .version
                .to_string(),
            "0.10.0"
        );
        assert!(plan_release(release("0.2.0-beta.1", b"MZfixture"), "0.1.0").is_err());
    }
    #[test]
    fn rejects_missing_duplicate_untrusted_and_oversize_assets() {
        let base = release("0.2.0", b"MZfixture");
        let mut missing = base.clone();
        missing.assets.pop();
        assert!(plan_release(missing, "0.1.0").is_err());
        let mut duplicate = base.clone();
        duplicate.assets.push(duplicate.assets[0].clone());
        assert!(plan_release(duplicate, "0.1.0").is_err());
        let mut foreign = base.clone();
        foreign.assets[0].browser_download_url = "https://example.com/file.exe".into();
        assert!(plan_release(foreign, "0.1.0").is_err());
        let mut huge = base;
        huge.assets[0].size = MAX_BINARY_SIZE + 1;
        assert!(plan_release(huge, "0.1.0").is_err());
    }
    #[test]
    fn manifest_requires_one_valid_exact_filename() {
        let hash = "a".repeat(64);
        assert_eq!(
            checksum_for(&format!("{hash} *{WINDOWS_ASSET}\r\n"), WINDOWS_ASSET).unwrap(),
            hash
        );
        assert!(checksum_for(&format!("{hash} other.exe"), WINDOWS_ASSET).is_err());
        assert!(
            checksum_for(
                &format!("{hash} {WINDOWS_ASSET}\n{hash} {WINDOWS_ASSET}"),
                WINDOWS_ASSET
            )
            .is_err()
        );
        assert!(checksum_for(&format!("xyz {WINDOWS_ASSET}"), WINDOWS_ASSET).is_err());
    }
    #[test]
    fn verified_download_is_staged_before_replacement() {
        let bytes = b"MZfixture";
        let mut future = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        future.patch += 1;
        let plan = plan_release(
            release(&future.to_string(), bytes),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap()
        .unwrap();
        let hash = format!("{:x}", Sha256::digest(bytes));
        let fetch = Fixture(HashMap::from([
            (plan.binary.browser_download_url.clone(), bytes.to_vec()),
            (
                plan.manifest.browser_download_url.clone(),
                format!("{hash} {WINDOWS_ASSET}\n").into_bytes(),
            ),
        ]));
        let called = Cell::new(false);
        install_using(
            &fetch,
            &plan,
            |_, _| Ok(()),
            |path| {
                assert_eq!(std::fs::read(path).unwrap(), bytes);
                called.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(called.get());
    }
    #[test]
    fn tampered_download_never_reaches_replacement() {
        let mut future = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        future.patch += 1;
        let plan = plan_release(
            release(&future.to_string(), b"MZfixture"),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap()
        .unwrap();
        let fetch = Fixture(HashMap::from([
            (
                plan.binary.browser_download_url.clone(),
                b"MZchanged".to_vec(),
            ),
            (
                plan.manifest.browser_download_url.clone(),
                format!("{} {WINDOWS_ASSET}\n", "a".repeat(64)).into_bytes(),
            ),
        ]));
        let called = Cell::new(false);
        let error = install_with(&fetch, &plan, |_| {
            called.set(true);
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("SHA-256 verification failed"), "{error}");
        assert!(!called.get());
    }
    #[test]
    fn github_digest_size_and_executable_type_are_all_enforced() {
        let bytes = b"MZfixture";
        let mut plan = plan_release(release("0.2.0", bytes), "0.1.0")
            .unwrap()
            .unwrap();
        let hash = format!("{:x}", Sha256::digest(bytes));
        plan.binary.digest = Some(format!("sha256:{}", "0".repeat(64)));
        assert!(verify_payload(&plan, bytes, &hash).is_err());
        plan.binary.digest = None;
        assert!(verify_payload(&plan, b"MZshort", &hash).is_err());
        let other = b"not-an-exe";
        plan.binary.size = other.len() as u64;
        assert!(verify_payload(&plan, other, &format!("{:x}", Sha256::digest(other))).is_err());
    }
}
