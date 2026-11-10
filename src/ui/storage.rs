use rustcalc::{history, model::Notebook};
use std::{collections::BTreeMap, io::ErrorKind, path::PathBuf};

const NOTEBOOK: &str = "notebook-v1";

/// Keep the existing location and on-disk representation so older builds can
/// still read a notebook after a binary rollback.
pub(super) struct Storage {
    path: PathBuf,
    values: BTreeMap<String, String>,
}

impl Storage {
    pub(super) fn open() -> Result<(Self, Notebook), String> {
        let dirs = directories::ProjectDirs::from("", "", "RustCalc")
            .ok_or("Could not locate the RustCalc data directory.")?;
        Self::load(dirs.data_dir().join("app.ron"))
    }

    fn load(path: PathBuf) -> Result<(Self, Notebook), String> {
        let values: BTreeMap<String, String> = match std::fs::read_to_string(&path) {
            Ok(raw) => ron::from_str(&raw).map_err(|e| e.to_string())?,
            Err(error) if error.kind() == ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error.to_string()),
        };
        let notebook = values
            .get(NOTEBOOK)
            .map(|raw| ron::from_str(raw).map_err(|e| e.to_string()))
            .transpose()?
            .unwrap_or_default();
        Ok((Self { path, values }, notebook))
    }

    pub(super) fn save(&mut self, notebook: &Notebook) -> Result<(), String> {
        let value = ron::to_string(notebook).map_err(|e| e.to_string())?;
        if self.values.get(NOTEBOOK) == Some(&value) {
            return Ok(());
        }
        let mut values = self.values.clone();
        values.insert(NOTEBOOK.into(), value);
        let bytes = ron::to_string(&values).map_err(|e| e.to_string())?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        history::write_file(&self.path, bytes.as_bytes())?;
        self.values = values;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_notebook_round_trips_without_discarding_other_eframe_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.ron");
        let old = r#"{"notebook-v1":"(answer:\"42\",language:Turkish)", "window":"old window"}"#;
        std::fs::write(&path, old).unwrap();
        let (mut storage, mut notebook) = Storage::load(path.clone()).unwrap();
        assert_eq!(notebook.answer, "42");
        assert_eq!(notebook.language, rustcalc::i18n::Language::Turkish);
        notebook.calculate("ans + 1").unwrap();
        storage.save(&notebook).unwrap();
        let (saved, notebook) = Storage::load(path).unwrap();
        assert_eq!(notebook.answer, "43");
        assert_eq!(notebook.entries[0].result, "43");
        assert_eq!(saved.values["window"], "old window");
    }

    #[test]
    fn unreadable_notebook_is_left_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.ron");
        std::fs::write(&path, "unreadable").unwrap();
        assert!(Storage::load(path.clone()).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "unreadable");
    }
}
