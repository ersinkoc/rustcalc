//! A release can have internally consistent hashes while accidentally containing
//! an executable from a different version. Never replace the app in that case.
use rustcalc::updater::{self, ASSET, Asset, EXECUTABLE_MAGIC, Fetch, REPOSITORY, Release};
use sha2::{Digest, Sha256};
use std::{cell::Cell, collections::HashMap};

struct Fixture(HashMap<String, Vec<u8>>);
impl Fetch for Fixture {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let bytes = self.0.get(url).ok_or("Unknown fixture URL")?;
        if bytes.len() as u64 > limit {
            return Err("Response too large".into());
        }
        Ok(bytes.clone())
    }
}
fn attempt(bytes: &[u8]) -> (Result<(), String>, bool) {
    let version = "999.0.0";
    let base = format!("https://github.com/{REPOSITORY}/releases/download/v{version}");
    let hash = format!("{:x}", Sha256::digest(bytes));
    let manifest = format!("{hash}  {ASSET}\n").into_bytes();
    let binary_url = format!("{base}/{ASSET}");
    let manifest_url = format!("{base}/SHA256SUMS");
    let plan = updater::plan_release(
        Release {
            tag_name: format!("v{version}"),
            draft: false,
            prerelease: false,
            assets: vec![
                Asset {
                    name: ASSET.into(),
                    browser_download_url: binary_url.clone(),
                    size: bytes.len() as u64,
                    digest: Some(format!("sha256:{hash}")),
                },
                Asset {
                    name: "SHA256SUMS".into(),
                    browser_download_url: manifest_url.clone(),
                    size: manifest.len() as u64,
                    digest: None,
                },
            ],
        },
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap()
    .unwrap();
    let fetch = Fixture(HashMap::from([
        (binary_url, bytes.to_vec()),
        (manifest_url, manifest),
    ]));
    let replaced = Cell::new(false);
    let result = updater::install_with(&fetch, &plan, |_| {
        replaced.set(true);
        Ok(())
    });
    (result, replaced.get())
}

#[test]
fn a_correctly_hashed_executable_from_the_wrong_version_is_rejected() {
    let binary = std::fs::read(env!("CARGO_BIN_EXE_rustcalc")).unwrap();
    let (result, replaced) = attempt(&binary);
    assert!(
        result.is_err(),
        "a matching checksum must not bypass executable version verification"
    );
    assert_eq!(
        result.unwrap_err(),
        "The downloaded executable does not match the release version"
    );
    assert!(!replaced, "the installed application must not be replaced");
}

#[test]
fn a_correctly_hashed_mz_file_that_is_not_executable_is_rejected() {
    let (result, replaced) = attempt(&[EXECUTABLE_MAGIC, b"not-a-real-executable"].concat());
    assert!(result.is_err(), "MZ alone does not prove an asset can run");
    assert!(!replaced);
}

#[test]
fn the_actual_application_passes_its_own_version_probe() {
    updater::verify_candidate_version(
        std::path::Path::new(env!("CARGO_BIN_EXE_rustcalc")),
        &semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
    )
    .unwrap();
}

fn installed_copy() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir
        .path()
        .join(format!("rustcalc{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_rustcalc"), &exe).unwrap();
    (dir, exe)
}
fn copy_over(target: &std::path::Path) -> impl FnOnce(&std::path::Path) -> Result<(), String> {
    let target = target.to_path_buf();
    move |staged| {
        std::fs::copy(staged, &target)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[test]
fn rollback_restores_the_kept_copy_and_can_be_undone() {
    let (_dir, exe) = installed_copy();
    let previous = updater::previous_path(&exe);
    assert_eq!(
        previous.file_name().unwrap().to_str().unwrap(),
        format!("rustcalc.previous{}", std::env::consts::EXE_SUFFIX)
    );
    // Simulate the state after an update: a runnable previous copy beside a
    // different current file.
    std::fs::copy(&exe, &previous).unwrap();
    let marker = b"newer build placeholder".to_vec();
    let real = std::fs::read(&exe).unwrap();
    std::fs::write(&exe, &marker).unwrap();
    let restored = updater::rollback_with(&exe, copy_over(&exe)).unwrap();
    assert_eq!(restored, env!("CARGO_PKG_VERSION"));
    assert_eq!(
        std::fs::read(&exe).unwrap(),
        real,
        "previous build restored"
    );
    assert_eq!(
        std::fs::read(&previous).unwrap(),
        marker,
        "the replaced build is kept for redo"
    );
}

#[test]
fn rollback_without_a_runnable_copy_changes_nothing() {
    let (_dir, exe) = installed_copy();
    let original = std::fs::read(&exe).unwrap();
    assert_eq!(
        updater::rollback_with(&exe, copy_over(&exe)).unwrap_err(),
        "No previous version is available to restore"
    );
    std::fs::write(updater::previous_path(&exe), b"MZbroken").unwrap();
    assert_eq!(
        updater::rollback_with(&exe, |_| panic!("must not replace")).unwrap_err(),
        "The previous version could not be started, so it was not restored"
    );
    assert_eq!(std::fs::read(&exe).unwrap(), original);
    assert_eq!(
        std::fs::read(updater::previous_path(&exe)).unwrap(),
        b"MZbroken"
    );
}
