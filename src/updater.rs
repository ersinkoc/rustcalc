//! Opt-in GitHub Releases updates. Plans retain one immutable tag; payloads must
//! pass both manifest and optional GitHub digest checks before replacement.
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const REPOSITORY: &str = "ersinkoc/rustcalc";
pub const RELEASES_URL: &str = "https://github.com/ersinkoc/rustcalc/releases";
pub const API_URL: &str = "https://api.github.com/repos/ersinkoc/rustcalc/releases/latest";
pub const WINDOWS_ASSET: &str = "rustcalc-windows-x86_64.exe";
pub const LINUX_ASSET: &str = "rustcalc-linux-x86_64";
pub const MACOS_ARM_ASSET: &str = "rustcalc-macos-aarch64";
pub const MACOS_INTEL_ASSET: &str = "rustcalc-macos-x86_64";

/// The release binary this build updates itself from, or `None` on platforms
/// without official binaries.
pub const PLATFORM_ASSET: Option<&str> = if cfg!(all(windows, target_arch = "x86_64")) {
    Some(WINDOWS_ASSET)
} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some(LINUX_ASSET)
} else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Some(MACOS_ARM_ASSET)
} else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
    Some(MACOS_INTEL_ASSET)
} else {
    None
};
/// The asset name plans are built for on this platform.
pub const ASSET: &str = match PLATFORM_ASSET {
    Some(name) => name,
    None => WINDOWS_ASSET,
};
/// Leading bytes of a native executable: PE on Windows, Mach-O on macOS, ELF
/// elsewhere. A release asset without them is rejected before it is run.
pub const EXECUTABLE_MAGIC: &[u8] = if cfg!(windows) {
    b"MZ"
} else if cfg!(target_os = "macos") {
    &[0xcf, 0xfa, 0xed, 0xfe]
} else {
    b"\x7fELF"
};
pub const MAX_BINARY_SIZE: u64 = 64 * 1024 * 1024;

/// Background update behaviour. Off unless the user chooses otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoUpdate {
    #[default]
    Off,
    /// Check periodically and announce a new release.
    Notify,
    /// Check periodically, then download, verify and stage the new release; it
    /// takes effect after the next restart.
    Install,
}

/// Automatic checks run at most once a day.
pub const AUTO_CHECK_INTERVAL: u64 = 24 * 60 * 60;

/// Whether an automatic check should start now. A clock that moved backwards
/// counts as due so a bad timestamp cannot suppress checks indefinitely.
pub fn auto_check_due(mode: AutoUpdate, last_check: u64, now: u64) -> bool {
    mode != AutoUpdate::Off && (now < last_check || now - last_check >= AUTO_CHECK_INTERVAL)
}

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
    let binary = select(ASSET, MAX_BINARY_SIZE)?;
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
    match PLATFORM_ASSET {
        Some(_) => Ok(()),
        None => Err(
            "Updates are not available for this platform. Download a release or build from source."
                .into(),
        ),
    }
}

/// Give a staged or restored file the execute permission Unix requires.
/// Lower-case hexadecimal SHA-256, as written in `SHA256SUMS`.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Prepares a staged file to become the program: executable on Unix, and on
/// Windows without the temporary-file attribute the staging file was created
/// with, so the installed program is an ordinary file.
fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, SetFileAttributesW};
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call.
        if unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_NORMAL) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
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
    let actual = sha256_hex(bytes);
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
    // Mach-O universal binaries start with their own magic.
    let universal = cfg!(target_os = "macos") && bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe]);
    if !bytes.starts_with(EXECUTABLE_MAGIC) && !universal {
        return Err("The release asset is not an executable for this system".into());
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
    // Stage beside the program: the candidate must be runnable (a noexec /tmp
    // cannot run it) and, on Unix, renamed over the program in one step.
    let builder = || {
        let mut builder = tempfile::Builder::new();
        builder
            .prefix(".rustcalc-update-")
            .suffix(std::env::consts::EXE_SUFFIX);
        builder
    };
    let mut staged = running_executable()
        .ok()
        .and_then(|exe| builder().tempfile_in(exe.parent()?).ok())
        .map_or_else(|| builder().tempfile(), Ok)
        .map_err(|e| e.to_string())?;
    staged.write_all(&bytes).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    make_executable(staged.path())?;
    // Close the file handle before Windows opens it for replacement.
    let staged = staged.into_temp_path();
    verify(&staged, &plan.version)?;
    replace(&staged)
}

/// Check the already hash-verified candidate without launching its GUI. Version
/// output is bounded; a hung or noisy process cannot stall the updater forever.
pub fn verify_candidate_version(path: &Path, expected: &Version) -> Result<(), String> {
    let output = probe_version(path)?;
    if output != format!("rustcalc {expected}") {
        return Err("The downloaded executable does not match the release version".into());
    }
    Ok(())
}

/// The trimmed `--version` output of an executable.
fn probe_version(path: &Path) -> Result<String, String> {
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
    match std::str::from_utf8(&output) {
        Ok(text) if status.success() && output.len() <= 128 => Ok(text.trim().to_owned()),
        _ => Err("The downloaded executable does not match the release version".into()),
    }
}

/// Where an update keeps the executable it replaced, in the same folder:
/// `rustcalc.exe` keeps `rustcalc.previous.exe`, `rustcalc` keeps
/// `rustcalc.previous`.
pub fn previous_path(exe: &Path) -> PathBuf {
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("rustcalc");
    exe.with_file_name(format!("{stem}.previous{}", std::env::consts::EXE_SUFFIX))
}

/// Copy the running executable aside before an update replaces it. A failed
/// copy does not block the update; it only means there is nothing to restore.
fn keep_previous(exe: &Path) -> Result<(), String> {
    // fs::copy keeps the execute permission on Unix.
    std::fs::copy(exe, previous_path(exe))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Swap `exe` with the copy kept by the last update and return the restored
/// version. The swapped-out executable becomes the new previous copy, so a
/// rollback can itself be undone. Nothing changes if the copy cannot run.
pub fn rollback_with(
    exe: &Path,
    replace: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<String, String> {
    let previous = previous_path(exe);
    if !previous.is_file() {
        return Err("No previous version is available to restore".into());
    }
    let restored = probe_version(&previous)
        .ok()
        .and_then(|output| output.strip_prefix("rustcalc ").map(str::to_owned))
        .filter(|version| Version::parse(version).is_ok())
        .ok_or("The previous version could not be started, so it was not restored")?;
    let current = std::fs::read(exe).map_err(|e| e.to_string())?;
    let directory = exe.parent().unwrap_or(Path::new("."));
    let mut staged = tempfile::Builder::new()
        .prefix("rustcalc-rollback-")
        .suffix(std::env::consts::EXE_SUFFIX)
        .tempfile_in(directory)
        .map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&previous).map_err(|e| e.to_string())?;
    staged.write_all(&bytes).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    make_executable(staged.path())?;
    let staged = staged.into_temp_path();
    replace(&staged)?;
    crate::history::write_file(&previous, &current)
        .and_then(|_| make_executable(&previous))
        .map_err(|_| "Restored, but the newer version could not be kept for redo".to_owned())?;
    Ok(restored)
}

/// The real path of the running program, captured on first use (main calls
/// this at startup). A command reached through a symlink (such as
/// `~/.local/bin/rustcalc`) resolves to the installed file. Linux reports a
/// replaced program as `… (deleted)`, so the path must be known before any
/// update replaces it.
pub fn running_executable() -> Result<PathBuf, String> {
    static EXECUTABLE: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    EXECUTABLE
        .get_or_init(|| {
            let exe = std::env::current_exe().ok()?;
            Some(exe.canonicalize().unwrap_or(exe))
        })
        .clone()
        .ok_or_else(|| "Could not find the running executable".to_owned())
}

/// Puts a staged executable in place of the running one. Unix renames over
/// the remembered path (atomic, and correct after an earlier update in this
/// session); Windows needs self-replace to move the locked file aside.
fn replace_running(staged: &Path) -> Result<(), String> {
    let failed = |e: &dyn std::fmt::Display| {
        format!("Could not replace the executable; close other RustCalc windows and retry: {e}")
    };
    #[cfg(unix)]
    {
        let exe = running_executable()?;
        if std::fs::rename(staged, &exe).is_ok() {
            return Ok(());
        }
        // Staged on another file system: copy beside the program, then rename.
        let directory = exe.parent().unwrap_or(Path::new("."));
        let copy = tempfile::Builder::new()
            .prefix(".rustcalc-replace-")
            .tempfile_in(directory)
            .map_err(|e| failed(&e))?
            .into_temp_path();
        std::fs::copy(staged, &copy).map_err(|e| failed(&e))?;
        make_executable(&copy)?;
        std::fs::rename(&copy, &exe).map_err(|e| failed(&e))
    }
    #[cfg(not(unix))]
    self_replace::self_replace(staged).map_err(|e| failed(&e))
}

pub fn rollback() -> Result<String, String> {
    ensure_supported_platform()?;
    let exe = running_executable()?;
    rollback_with(&exe, replace_running)
}
pub fn install(plan: &UpdatePlan) -> Result<(), String> {
    ensure_supported_platform()?;
    install_with(&GitHub, plan, |path| {
        if let Ok(exe) = running_executable() {
            let _ = keep_previous(&exe);
        }
        replace_running(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sha256_is_lower_case_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[cfg(windows)]
    #[test]
    fn staged_programs_lose_the_temporary_attribute() {
        use std::os::windows::fs::MetadataExt;
        const TEMPORARY: u32 = 0x100;
        let staged = tempfile::Builder::new().tempfile().unwrap().into_temp_path();
        let before = std::fs::metadata(&staged).unwrap().file_attributes();
        assert_ne!(before & TEMPORARY, 0, "tempfile marks staging files temporary");
        make_executable(&staged).unwrap();
        let attributes = std::fs::metadata(&staged).unwrap().file_attributes();
        assert_eq!(attributes & TEMPORARY, 0, "attributes {attributes:#x}");
    }
    /// Fixture bytes that pass this platform's executable header check.
    fn exe(tail: &[u8]) -> Vec<u8> {
        [EXECUTABLE_MAGIC, tail].concat()
    }
    use std::{cell::Cell, collections::HashMap};
    #[test]
    fn automatic_checks_are_opt_in_and_run_at_most_daily() {
        let day = AUTO_CHECK_INTERVAL;
        assert!(!auto_check_due(AutoUpdate::Off, 0, 10 * day));
        for mode in [AutoUpdate::Notify, AutoUpdate::Install] {
            assert!(auto_check_due(mode, 0, 1_800_000_000));
            assert!(!auto_check_due(mode, 1_000, 1_000 + day - 1));
            assert!(auto_check_due(mode, 1_000, 1_000 + day));
            assert!(auto_check_due(mode, 5_000, 1_000), "clock moved backwards");
        }
        assert_eq!(AutoUpdate::default(), AutoUpdate::Off);
    }
    fn release(version: &str, bytes: &[u8]) -> Release {
        Release {
            tag_name: format!("v{version}"),
            draft: false,
            prerelease: false,
            assets: vec![
                Asset {
                    name: ASSET.into(),
                    browser_download_url: format!(
                        "https://github.com/{REPOSITORY}/releases/download/v{version}/{ASSET}"
                    ),
                    size: bytes.len() as u64,
                    digest: Some(format!("sha256:{}", sha256_hex(bytes))),
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
            plan_release(release("0.1.0", &exe(b"fixture")), "0.1.0")
                .unwrap()
                .is_none()
        );
        assert!(
            plan_release(release("0.0.9", &exe(b"fixture")), "0.1.0")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            plan_release(release("0.10.0", &exe(b"fixture")), "0.9.0")
                .unwrap()
                .unwrap()
                .version
                .to_string(),
            "0.10.0"
        );
        assert!(plan_release(release("0.2.0-beta.1", &exe(b"fixture")), "0.1.0").is_err());
    }
    #[test]
    fn rejects_missing_duplicate_untrusted_and_oversize_assets() {
        let base = release("0.2.0", &exe(b"fixture"));
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
            checksum_for(&format!("{hash} *{ASSET}\r\n"), ASSET).unwrap(),
            hash
        );
        assert!(checksum_for(&format!("{hash} other.exe"), ASSET).is_err());
        assert!(checksum_for(&format!("{hash} {ASSET}\n{hash} {ASSET}"), ASSET).is_err());
        assert!(checksum_for(&format!("xyz {ASSET}"), ASSET).is_err());
    }
    #[test]
    fn verified_download_is_staged_before_replacement() {
        let bytes: &[u8] = &exe(b"fixture");
        let mut future = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        future.patch += 1;
        let plan = plan_release(
            release(&future.to_string(), bytes),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap()
        .unwrap();
        let hash = sha256_hex(bytes);
        let fetch = Fixture(HashMap::from([
            (plan.binary.browser_download_url.clone(), bytes.to_vec()),
            (
                plan.manifest.browser_download_url.clone(),
                format!("{hash} {ASSET}\n").into_bytes(),
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
            release(&future.to_string(), &exe(b"fixture")),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap()
        .unwrap();
        let fetch = Fixture(HashMap::from([
            (plan.binary.browser_download_url.clone(), exe(b"changed")),
            (
                plan.manifest.browser_download_url.clone(),
                format!("{} {ASSET}\n", "a".repeat(64)).into_bytes(),
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
        let bytes: &[u8] = &exe(b"fixture");
        let mut plan = plan_release(release("0.2.0", bytes), "0.1.0")
            .unwrap()
            .unwrap();
        let hash = sha256_hex(bytes);
        plan.binary.digest = Some(format!("sha256:{}", "0".repeat(64)));
        assert!(verify_payload(&plan, bytes, &hash).is_err());
        plan.binary.digest = None;
        assert!(verify_payload(&plan, &exe(b"short"), &hash).is_err());
        let other = b"not-an-exe";
        plan.binary.size = other.len() as u64;
        assert!(verify_payload(&plan, other, &sha256_hex(other)).is_err());
    }
}
