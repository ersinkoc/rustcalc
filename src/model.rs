use crate::engine::{self, Angle, Context, Value};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

pub const HISTORY_LIMIT: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub expression: String,
    pub result: String,
    pub approximate: bool,
    pub angle: Angle,
    pub pinned: bool,
    pub timestamp: u64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Notebook {
    pub language: crate::i18n::Language,
    pub entries: Vec<Entry>,
    pub memory: String,
    pub answer: String,
    pub answer_approximate: bool,
    pub memory_approximate: bool,
    pub angle: Angle,
    pub light: bool,
    pub auto_update: crate::updater::AutoUpdate,
    /// Unix seconds of the last automatic check attempt.
    pub last_update_check: u64,
    /// Whether the one-time automatic update question has been answered.
    pub update_prompted: bool,
    /// User formulas; built-in templates are bundled, not stored.
    pub formulas: Vec<crate::formulas::Formula>,
    /// Plot functions f, g and h; `None` keeps the default example.
    pub plots: Option<[String; 3]>,
    pub programmer_bits: Option<u32>,
    pub programmer_signed: bool,
    /// The workspace, keypad, converter units and formula open at the last exit.
    pub workspace: String,
    pub scientific: bool,
    pub converter: Option<[usize; 3]>,
    pub formula_selected: Option<String>,
    #[serde(skip)]
    history_undo: Vec<Vec<(usize, Entry)>>,
}
impl Notebook {
    pub fn context(&self) -> Context {
        Context {
            angle: self.angle,
            memory: Decimal::from_str(&self.memory).unwrap_or_default(),
            ans: Decimal::from_str(&self.answer).unwrap_or_default(),
            ans_approximate: self.answer_approximate,
            memory_approximate: self.memory_approximate,
            ..Context::default()
        }
    }
    pub fn calculate(&mut self, expression: &str) -> Result<Value, engine::CalcError> {
        let value = engine::evaluate(expression, self.context())?;
        self.answer = value.exact_text();
        self.answer_approximate = value.approximate;
        if self.entries.last().is_none_or(|e| {
            e.expression != expression || e.result != value.text() || e.angle != self.angle
        }) {
            self.entries.push(Entry {
                expression: expression.into(),
                result: value.text(),
                approximate: value.approximate,
                angle: self.angle,
                pinned: false,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }
        // Pinned entries survive automatic pruning; the total remains bounded.
        while self.entries.len() > HISTORY_LIMIT {
            let index = self.entries.iter().position(|e| !e.pinned).unwrap_or(0);
            self.entries.remove(index);
        }
        Ok(value)
    }
    fn remember_removed(&mut self, removed: Vec<(usize, Entry)>) {
        if removed.is_empty() {
            return;
        }
        self.history_undo.push(removed);
        if self.history_undo.len() > 20 {
            self.history_undo.remove(0);
        }
    }
    pub fn delete_history(&mut self, index: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        let entry = self.entries.remove(index);
        self.remember_removed(vec![(index, entry)]);
        true
    }
    pub fn clear_unpinned_history(&mut self) {
        let removed = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.pinned)
            .map(|(index, entry)| (index, entry.clone()))
            .collect();
        self.entries.retain(|entry| entry.pinned);
        self.remember_removed(removed);
    }
    pub fn can_undo_history(&self) -> bool {
        !self.history_undo.is_empty()
    }
    pub fn undo_history(&mut self) -> Result<bool, String> {
        let Some(removed) = self.history_undo.last() else {
            return Ok(false);
        };
        if self.entries.len() + removed.len() > HISTORY_LIMIT {
            return Err("Undo would exceed the 500-entry history limit.".into());
        }
        let removed = self.history_undo.pop().unwrap();
        for (index, entry) in removed {
            self.entries.insert(index.min(self.entries.len()), entry);
        }
        Ok(true)
    }
    /// Merge stored results without evaluating expressions in a new ans/mem context.
    pub fn import_history(&mut self, incoming: &[Entry]) -> Result<usize, String> {
        crate::history::validate_entries(incoming)?;
        let mut merged = self.entries.clone();
        let mut matched = vec![false; self.entries.len()];
        let mut added = 0;
        for entry in incoming {
            // Match each existing occurrence once. Repeated calculations in a
            // backup are distinct events even when their second-level times match.
            if let Some((index, existing)) =
                merged
                    .iter_mut()
                    .enumerate()
                    .take(matched.len())
                    .find(|(index, existing)| {
                        !matched[*index]
                            && existing.expression == entry.expression
                            && existing.result == entry.result
                            && existing.approximate == entry.approximate
                            && existing.angle == entry.angle
                            && existing.timestamp == entry.timestamp
                    })
            {
                matched[index] = true;
                existing.pinned |= entry.pinned;
            } else {
                merged.push(entry.clone());
                added += 1;
            }
        }
        if merged.len() > HISTORY_LIMIT {
            return Err("The imported history would exceed 500 entries.".into());
        }
        if self.entries != merged {
            // Import establishes a new baseline; old deletion positions would
            // otherwise reintroduce duplicates from the restored backup.
            self.history_undo.clear();
            self.entries = merged;
        }
        Ok(added)
    }
    pub fn memory_add(&mut self, value: Value, subtract: bool) -> Result<(), String> {
        let current = self.context().memory;
        let next = if subtract {
            current.checked_sub(value.number)
        } else {
            current.checked_add(value.number)
        }
        .ok_or("Memory is outside the supported number range")?;
        self.memory = next.normalize().to_string();
        self.memory_approximate |= value.approximate;
        Ok(())
    }
    pub fn export(&self) -> String {
        self.entries
            .iter()
            .map(|e| {
                format!(
                    "{} = {}{} [{}]\n",
                    e.expression,
                    if e.approximate { "≈ " } else { "" },
                    e.result,
                    if e.angle == Angle::Degrees {
                        "DEG"
                    } else {
                        "RAD"
                    }
                )
            })
            .collect()
    }
}
