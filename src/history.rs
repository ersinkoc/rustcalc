//! Versioned, bounded history backups. Import never executes stored expressions.
use crate::model::{Entry, HISTORY_LIMIT};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    format: String,
    version: u32,
    entries: Vec<Entry>,
}

pub fn validate_entries(entries: &[Entry]) -> Result<(), String> {
    if entries.len() > HISTORY_LIMIT {
        return Err("A history backup can contain at most 500 entries.".into());
    }
    for entry in entries {
        if entry.expression.trim().is_empty()
            || entry.expression.chars().count() > 4096
            || entry.expression.chars().any(char::is_control)
            || entry.result.len() > 128
            || Decimal::from_str_exact(&entry.result).is_err()
        {
            return Err("The backup contains an invalid calculation.".into());
        }
    }
    Ok(())
}

pub fn encode(entries: &[Entry]) -> Result<Vec<u8>, String> {
    validate_entries(entries)?;
    let bytes = serde_json::to_vec_pretty(&Backup {
        format: "rustcalc-history".into(),
        version: 1,
        entries: entries.to_vec(),
    })
    .map_err(|_| "Could not encode the history backup.".to_owned())?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err("The history backup exceeds the 4 MiB limit.".into());
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Entry>, String> {
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err("The history backup exceeds the 4 MiB limit.".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let backup: Backup = serde_json::from_slice(bytes)
        .map_err(|_| "This file is not a valid RustCalc history backup.".to_owned())?;
    if backup.format != "rustcalc-history" || backup.version != 1 {
        return Err("This history backup format or version is not supported.".into());
    }
    validate_entries(&backup.entries)?;
    Ok(backup.entries)
}

pub fn read_backup(path: &Path) -> Result<Vec<Entry>, String> {
    let file =
        std::fs::File::open(path).map_err(|_| "Could not open the history backup.".to_owned())?;
    let mut bytes = Vec::new();
    file.take(MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the history backup.".to_owned())?;
    decode(&bytes)
}

/// Save beside the destination and atomically replace it only after a full write.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Could not create the history file.".to_owned())?;
    staged
        .write_all(bytes)
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|_| "Could not write the history file.".to_owned())?;
    staged
        .persist(path)
        .map_err(|_| "Could not save the history file. The original file was kept.".to_owned())?;
    Ok(())
}
