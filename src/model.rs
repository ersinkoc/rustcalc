use crate::engine::{self, Angle, Context, Value};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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

/// A value the user named with `name = expression`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    /// Canonical decimal text with every stored digit.
    pub value: String,
    pub approximate: bool,
}

/// At most this many named values are kept.
pub const VARIABLE_LIMIT: usize = 100;

/// Splits `name = expression` into its parts. `==`, `<=`, `>=` and `!=` are
/// comparisons, not assignments.
pub fn assignment(expression: &str) -> Option<(&str, &str)> {
    let at = expression.find('=')?;
    let (name, rest) = (&expression[..at], &expression[at + 1..]);
    if rest.starts_with('=') || name.ends_with(['<', '>', '!']) {
        return None;
    }
    let name = name.trim();
    engine::is_identifier(name).then_some((name, rest))
}

/// The expression with missing closing brackets added at the end, as
/// calculators do: `sqrt(16` is `sqrt(16)`. An extra `)` is left for the
/// parser to report.
pub fn complete_brackets(expression: &str) -> std::borrow::Cow<'_, str> {
    let mut depth = 0usize;
    for c in expression.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return expression.into(),
            ')' => depth -= 1,
            _ => {}
        }
    }
    if depth == 0 || depth > 64 {
        expression.into()
    } else {
        format!("{}{}", expression.trim_end(), ")".repeat(depth)).into()
    }
}

/// Whether `name` may be assigned: an identifier that is not a constant,
/// function, `ans`, `mem` or the plot variable `x`.
pub fn check_variable_name(name: &str) -> Result<(), String> {
    if !engine::is_identifier(name) {
        return Err(format!("Invalid variable name: {name}"));
    }
    if engine::is_reserved(name) || name.to_lowercase() == "x" {
        return Err(format!("Reserved name: {name}"));
    }
    // `2e1` is scientific notation (20), so a name like e1 could not follow a
    // number: 2e1 would never mean 2 × e1.
    let mut chars = name.chars();
    if chars.next().is_some_and(|c| c.eq_ignore_ascii_case(&'e'))
        && chars.next().is_some_and(|c| c.is_ascii_digit())
    {
        return Err(format!("Reserved name: {name}"));
    }
    Ok(())
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
    /// Decimal mark, digit grouping and fixed decimals for displayed numbers.
    pub numbers: crate::numfmt::NumberStyle,
    /// Follow the operating system's light or dark setting instead of `light`.
    pub follow_system_theme: bool,
    /// White and yellow on black with strong outlines, chosen in Settings.
    pub high_contrast: bool,
    /// Interface scale; 0 (an older save) means 100%.
    pub zoom: f32,
    /// Named values from `name = expression`, by lower-case name.
    pub variables: BTreeMap<String, Variable>,
    /// Changed keyboard shortcuts by command; an empty text removes one.
    pub shortcuts: BTreeMap<String, String>,
    #[serde(skip)]
    history_undo: Vec<Vec<(usize, Entry)>>,
}
/// Interface scales offered in Settings and stepped through with Ctrl + / −.
pub const ZOOM_STEPS: [f32; 7] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75];

impl Notebook {
    pub fn zoom(&self) -> f32 {
        if self.zoom > 0.0 {
            self.zoom
                .clamp(ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1])
        } else {
            1.0
        }
    }
    /// The next larger (`step > 0`) or smaller scale.
    pub fn step_zoom(&mut self, step: i32) {
        let current = self.zoom();
        let next = if step > 0 {
            ZOOM_STEPS.iter().copied().find(|z| *z > current + 0.001)
        } else {
            ZOOM_STEPS
                .iter()
                .rev()
                .copied()
                .find(|z| *z < current - 0.001)
        };
        if let Some(next) = next {
            self.zoom = next;
        }
    }
    /// A canonical number as the user wants to read it.
    pub fn show(&self, canonical: &str) -> String {
        self.numbers.show(canonical, self.language)
    }
    /// A stored value as shown: approximate values with their 15 trustworthy
    /// digits, like a result.
    pub fn show_value(&self, canonical: &str, approximate: bool) -> String {
        let text = Decimal::from_str(canonical)
            .map(|number| {
                Value {
                    number,
                    approximate,
                }
                .text()
            })
            .unwrap_or_else(|_| canonical.to_owned());
        self.show(&text)
    }
    /// A canonical number as it goes to the clipboard.
    pub fn copy_text(&self, canonical: &str) -> String {
        self.numbers.copy(canonical, self.language)
    }
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
    /// The named values as the engine reads them.
    pub fn engine_variables(&self) -> engine::Variables {
        self.variables
            .iter()
            .filter_map(|(name, v)| {
                Some((
                    name.clone(),
                    Value {
                        number: Decimal::from_str(&v.value).ok()?,
                        approximate: v.approximate,
                    },
                ))
            })
            .collect()
    }
    /// Evaluates an expression or the right side of `name = expression`,
    /// returning the assigned name (lower case) with the value.
    pub fn evaluate(&self, expression: &str) -> Result<(Option<String>, Value), engine::CalcError> {
        let variables = self.engine_variables();
        let expression = &*complete_brackets(expression);
        if let Some((name, rest)) = assignment(expression) {
            check_variable_name(name).map_err(engine::CalcError::from)?;
            let value = engine::evaluate_with(rest, self.context(), &variables)?;
            return Ok((Some(name.to_lowercase()), value));
        }
        let value = engine::evaluate_with(expression, self.context(), &variables)?;
        Ok((None, value))
    }
    pub fn calculate(&mut self, expression: &str) -> Result<Value, engine::CalcError> {
        let (name, value) = self.evaluate(expression)?;
        // History keeps the expression as it was calculated, brackets closed.
        let expression = &*complete_brackets(expression);
        if let Some(name) = name {
            if !self.variables.contains_key(&name) && self.variables.len() >= VARIABLE_LIMIT {
                return Err(engine::CalcError::from(
                    "Too many variables; delete one first".to_owned(),
                ));
            }
            self.variables.insert(
                name,
                Variable {
                    value: value.exact_text(),
                    approximate: value.approximate,
                },
            );
        }
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
