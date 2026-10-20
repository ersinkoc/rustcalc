use crate::engine::{self, Angle, Context, Value};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
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
        self.answer = value.text();
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
        while self.entries.len() > 500 {
            let index = self.entries.iter().position(|e| !e.pinned).unwrap_or(0);
            self.entries.remove(index);
        }
        Ok(value)
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
