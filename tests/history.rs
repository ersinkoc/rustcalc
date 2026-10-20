use rustcalc::{engine::Angle, history, i18n::Language, model::Notebook};

fn sample() -> Notebook {
    let mut book = Notebook::default();
    book.calculate("2+3").unwrap();
    book.entries[0].pinned = true;
    book.angle = Angle::Radians;
    book.calculate("sqrt(2)").unwrap();
    book.calculate("ans*2").unwrap();
    book
}

#[test]
fn backup_round_trips_stored_results_and_metadata_without_recalculation() {
    let book = sample();
    let bytes = history::encode(&book.entries).unwrap();
    let entries = history::decode(&bytes).unwrap();
    assert_eq!(entries, book.entries);
    let mut target = Notebook::default();
    target.calculate("100").unwrap();
    target.memory = "42".into();
    target.language = Language::Turkish;
    target.light = true;
    assert_eq!(target.import_history(&entries).unwrap(), 3);
    assert_eq!(&target.entries[1..], entries);
    assert_eq!(target.answer, "100");
    assert_eq!(target.memory, "42");
    assert_eq!(target.angle, Angle::Degrees);
    assert_eq!(target.language, Language::Turkish);
    assert!(target.light);
    assert_eq!(target.import_history(&entries).unwrap(), 0);
    assert_eq!(target.entries.len(), 4);
}

#[test]
fn importing_repeated_calculations_preserves_their_multiplicity() {
    let original = sample().entries;
    // The same calculation can occur twice within the timestamp's one-second
    // resolution. A lossless backup must retain both occurrences.
    let entries = vec![
        original[0].clone(),
        original[1].clone(),
        original[0].clone(),
    ];
    let backup = history::decode(&history::encode(&entries).unwrap()).unwrap();
    let mut book = Notebook::default();
    assert_eq!(book.import_history(&backup).unwrap(), 3);
    assert_eq!(book.entries, entries);
    assert_eq!(book.import_history(&backup).unwrap(), 0);
    assert_eq!(book.entries, entries);
}

#[test]
fn import_merges_pins_and_starts_a_new_undo_baseline_only_when_changed() {
    let mut book = sample();
    let backup = book.entries.clone();
    book.delete_history(2);
    assert!(book.can_undo_history());
    assert_eq!(book.import_history(&backup).unwrap(), 1);
    assert!(!book.can_undo_history());
    assert_eq!(book.entries, backup);
    book.entries[0].pinned = false;
    assert_eq!(book.import_history(&backup).unwrap(), 0);
    assert!(book.entries[0].pinned);
    let mut unpinned = backup;
    unpinned[0].pinned = false;
    book.import_history(&unpinned).unwrap();
    assert!(book.entries[0].pinned);
    book.delete_history(2);
    let unchanged = book.entries.clone();
    book.import_history(&unchanged).unwrap();
    assert!(book.can_undo_history());
}

#[test]
fn delete_and_clear_undo_restore_order_and_keep_new_calculations() {
    let mut book = sample();
    let original = book.entries.clone();
    assert!(book.delete_history(1));
    book.calculate("27").unwrap();
    assert!(book.undo_history().unwrap());
    assert_eq!(&book.entries[..3], original);
    assert_eq!(book.entries[3].result, "27");
    book.clear_unpinned_history();
    assert_eq!(book.entries.len(), 1);
    assert!(book.entries[0].pinned);
    book.calculate("81").unwrap();
    assert!(book.undo_history().unwrap());
    assert_eq!(&book.entries[..3], original);
    assert_eq!(book.entries[3].result, "27");
    assert_eq!(book.entries[4].result, "81");
    assert_eq!(book.answer, "81");
    assert!(!book.undo_history().unwrap());
    assert!(!book.delete_history(999));
}

#[test]
fn history_undo_is_bounded_and_not_persisted() {
    let mut book = Notebook::default();
    for i in 0..25 {
        book.calculate(&i.to_string()).unwrap();
        book.delete_history(0);
    }
    let serialized = serde_json::to_string(&book).unwrap();
    let restored: Notebook = serde_json::from_str(&serialized).unwrap();
    assert!(!restored.can_undo_history());
    let mut undone = 0;
    while book.undo_history().unwrap() {
        undone += 1;
    }
    assert_eq!(undone, 20);
    assert_eq!(book.entries.len(), 20);
}

#[test]
fn capacity_errors_leave_history_pins_and_undo_unchanged() {
    let mut book = Notebook::default();
    for i in 0..500 {
        book.calculate(&i.to_string()).unwrap();
    }
    let mut incoming = vec![book.entries[0].clone()];
    incoming[0].pinned = true;
    let mut extra = book.entries[0].clone();
    extra.expression = "9001".into();
    incoming.push(extra);
    let original = book.entries.clone();
    assert!(
        book.import_history(&incoming)
            .unwrap_err()
            .contains("exceed 500")
    );
    assert_eq!(book.entries, original);
    book.delete_history(0);
    book.calculate("9001").unwrap();
    let before_undo = book.entries.clone();
    assert!(book.undo_history().unwrap_err().contains("500-entry"));
    assert!(book.can_undo_history());
    assert_eq!(book.entries, before_undo);
}

#[test]
fn malformed_unsupported_oversized_and_invalid_backups_are_rejected_atomically() {
    let bytes = history::encode(&sample().entries).unwrap();
    for invalid in [
        b"not json".to_vec(),
        b"{}".to_vec(),
        vec![b' '; history::MAX_BACKUP_BYTES as usize + 1],
    ] {
        assert!(history::decode(&invalid).is_err());
    }
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (field, value) in [
        ("version", serde_json::json!(99)),
        ("format", serde_json::json!("other")),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        assert!(
            history::decode(&serde_json::to_vec(&changed).unwrap())
                .unwrap_err()
                .contains("not supported")
        );
    }
    for result in [
        "NaN",
        "1e999",
        "not a result",
        "0.00000000000000000000000000001",
    ] {
        let mut entries = sample().entries;
        entries[0].result = result.into();
        let mut target = sample();
        let before = target.entries.clone();
        assert!(
            target.import_history(&entries).is_err(),
            "accepted {result}"
        );
        assert_eq!(target.entries, before);
    }
    for expression in ["".into(), "\n1+2".into(), "x".repeat(4097)] {
        let mut entries = sample().entries;
        entries[0].expression = expression;
        assert!(history::encode(&entries).is_err());
    }
    let entries = vec![sample().entries[0].clone(); 501];
    assert!(history::encode(&entries).is_err());
}

#[test]
fn real_file_round_trip_replacement_and_failed_save_preserve_original() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Türkçe 'quote $ history.json");
    std::fs::write(&path, b"old content").unwrap();
    let book = sample();
    let bytes = history::encode(&book.entries).unwrap();
    history::write_file(&path, &bytes).unwrap();
    assert_eq!(history::read_backup(&path).unwrap(), book.entries);
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&bytes);
    history::write_file(&path, &bom).unwrap();
    assert_eq!(history::read_backup(&path).unwrap(), book.entries);
    let target_directory = directory.path().join("existing directory");
    std::fs::create_dir(&target_directory).unwrap();
    std::fs::write(target_directory.join("kept.txt"), b"keep").unwrap();
    assert!(history::write_file(&target_directory, &bytes).is_err());
    assert_eq!(
        std::fs::read(target_directory.join("kept.txt")).unwrap(),
        b"keep"
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    std::fs::write(&path, vec![b' '; history::MAX_BACKUP_BYTES as usize + 1]).unwrap();
    assert!(history::read_backup(&path).unwrap_err().contains("4 MiB"));
    assert!(history::read_backup(&directory.path().join("missing.json")).is_err());
}

#[cfg(windows)]
#[test]
fn a_locked_destination_keeps_its_original_contents_and_leaves_no_staging_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("history.json");
    std::fs::write(&path, b"original backup").unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert!(
        history::write_file(&path, b"new backup")
            .unwrap_err()
            .contains("original file was kept")
    );
    drop(lock);
    assert_eq!(std::fs::read(&path).unwrap(), b"original backup");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
