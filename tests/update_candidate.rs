//! A release can have internally consistent hashes while accidentally containing
//! an executable from a different version. Never replace the app in that case.
use rustcalc::updater::{self, Asset, Fetch, REPOSITORY, Release, WINDOWS_ASSET};
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
    let manifest = format!("{hash}  {WINDOWS_ASSET}\n").into_bytes();
    let binary_url = format!("{base}/{WINDOWS_ASSET}");
    let manifest_url = format!("{base}/SHA256SUMS");
    let plan = updater::plan_release(
        Release {
            tag_name: format!("v{version}"),
            draft: false,
            prerelease: false,
            assets: vec![
                Asset {
                    name: WINDOWS_ASSET.into(),
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
    assert!(result.unwrap_err().contains("version"));
    assert!(!replaced, "the installed application must not be replaced");
}

#[test]
fn a_correctly_hashed_mz_file_that_is_not_executable_is_rejected() {
    let (result, replaced) = attempt(b"MZnot-a-real-Windows-executable");
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
