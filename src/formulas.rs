//! Reusable calculations: named inputs feed a sequence of result expressions.
//! Built-in templates and user formulas share one schema, so a template can be
//! duplicated and edited, and formulas can be shared as JSON files.
use crate::numfmt::NumberStyle;
use crate::{
    dates,
    engine::{self, Context, Expression, Value, Variables},
    i18n::Language,
};
use rust_decimal::{Decimal, RoundingStrategy, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};

pub const MAX_FIELDS: usize = 20;
pub const MAX_FORMULAS: usize = 200;
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
const FORMAT_NAME: &str = "rustcalc-formulas";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Number,
    Date,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Number,
    Money,
    Percent,
    Date,
    Weekday,
}
impl Format {
    pub const ALL: [Format; 5] = [
        Format::Number,
        Format::Money,
        Format::Percent,
        Format::Date,
        Format::Weekday,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Format::Number => "Number",
            Format::Money => "Money (2 decimals)",
            Format::Percent => "Percent",
            Format::Date => "Date",
            Format::Weekday => "Day of the week",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub kind: Kind,
    pub default: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Output {
    pub name: String,
    pub label: String,
    pub expression: String,
    pub format: Format,
    pub unit: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Formula {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub inputs: Vec<Field>,
    pub outputs: Vec<Output>,
}

/// The bundled templates, in display order.
pub fn builtins() -> &'static [Formula] {
    static BUILTINS: std::sync::OnceLock<Vec<Formula>> = std::sync::OnceLock::new();
    BUILTINS.get_or_init(|| {
        serde_json::from_str(include_str!("../templates/formulas.json"))
            .expect("valid bundled formula templates")
    })
}

pub fn is_builtin(id: &str) -> bool {
    builtins().iter().any(|f| f.id == id)
}

impl Formula {
    /// A blank formula with one input and one result, ready for editing, with
    /// its starting labels in `language`. An empty category lists as Custom.
    pub fn blank(language: Language) -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            category: String::new(),
            description: String::new(),
            inputs: vec![Field {
                name: "a".into(),
                label: "A".into(),
                kind: Kind::Number,
                default: "1".into(),
            }],
            outputs: vec![Output {
                name: "result".into(),
                label: language.text("Result"),
                expression: "a * 2".into(),
                format: Format::Number,
                unit: String::new(),
            }],
        }
    }
    /// An editable copy with its visible text in `language`.
    pub fn localized_copy(&self, language: Language) -> Self {
        let mut copy = self.clone();
        copy.id = String::new();
        copy.name = language.text(&self.name);
        copy.category = String::new();
        copy.description = language.text(&self.description);
        for field in &mut copy.inputs {
            field.label = language.text(&field.label);
        }
        for output in &mut copy.outputs {
            output.label = language.text(&output.label);
            output.unit = language.text(&output.unit);
        }
        copy
    }
    pub fn defaults(&self) -> Vec<String> {
        self.inputs.iter().map(|f| f.default.clone()).collect()
    }
}

/// Check structure, names and expressions. Messages are catalogue keys or
/// templates so the interface can translate them.
pub fn validate(formula: &Formula) -> Result<(), String> {
    if formula.name.trim().is_empty() {
        return Err("Enter a name for the formula".into());
    }
    if formula.outputs.is_empty() {
        return Err("Add at least one result".into());
    }
    if formula.inputs.len() > MAX_FIELDS || formula.outputs.len() > MAX_FIELDS {
        return Err("Too many inputs or results (maximum 20 each)".into());
    }
    let too_long = formula.name.chars().count() > 60
        || formula.category.chars().count() > 40
        || formula.description.chars().count() > 500
        || formula
            .inputs
            .iter()
            .any(|f| f.label.chars().count() > 80 || f.default.chars().count() > 200)
        || formula.outputs.iter().any(|o| {
            o.label.chars().count() > 80
                || o.unit.chars().count() > 20
                || o.expression.chars().count() > 1024
        });
    if too_long {
        return Err("Text is too long".into());
    }
    let mut known: Vec<String> = Vec::new();
    let mut declare = |name: &str| -> Result<(), String> {
        let lower = name.to_ascii_lowercase();
        if !engine::is_identifier(name) || name.len() > 32 {
            return Err(format!("Invalid variable name: {name}"));
        }
        if engine::is_reserved(&lower) {
            return Err(format!("Reserved name: {name}"));
        }
        if known.contains(&lower) {
            return Err(format!("Duplicate name: {name}"));
        }
        known.push(lower);
        Ok(())
    };
    for field in &formula.inputs {
        declare(&field.name)?;
    }
    for output in &formula.outputs {
        let expression = Expression::parse(&output.expression).map_err(|e| e.message)?;
        // Results may use inputs and earlier results, never later ones.
        let available: Vec<String> = known_names(formula, output);
        if let Some(unknown) = expression
            .names()
            .into_iter()
            .find(|n| !available.contains(n))
        {
            return Err(format!("Unknown name: {unknown}"));
        }
        declare(&output.name)?;
    }
    Ok(())
}

fn known_names(formula: &Formula, until: &Output) -> Vec<String> {
    let mut names: Vec<String> = formula
        .inputs
        .iter()
        .map(|f| f.name.to_ascii_lowercase())
        .collect();
    for output in &formula.outputs {
        if std::ptr::eq(output, until) {
            break;
        }
        names.push(output.name.to_ascii_lowercase());
    }
    names
}

/// The evaluated inputs and results of one formula.
/// Whether a typed number looks like it uses thousands separators, such as
/// `250.000` or `1,500`, which RustCalc reads as 250 and 1.5.
pub fn looks_grouped(text: &str) -> bool {
    let text = text.trim();
    let Some(separator) = text.chars().find(|c| matches!(c, '.' | ',')) else {
        return false;
    };
    let mut groups = text.split(separator);
    let first = groups.next().unwrap_or_default();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(first)
        && first.len() <= 3
        && !first.starts_with('0')
        && groups.all(|g| g.len() == 3 && digits(g))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub inputs: Vec<Result<Value, String>>,
    pub outputs: Vec<Result<Value, String>>,
}

/// Evaluate inputs in order, then each result with the inputs and earlier
/// results as variables. A result that depends on a failed value reports that
/// instead of a confusing unknown-name error.
pub fn run(formula: &Formula, values: &[String], context: Context) -> Run {
    let mut variables = Variables::new();
    let mut failed: Vec<String> = Vec::new();
    let mut inputs = Vec::with_capacity(formula.inputs.len());
    for (index, field) in formula.inputs.iter().enumerate() {
        let text = values.get(index).map(String::as_str).unwrap_or("").trim();
        let date_shaped = text
            .chars()
            .all(|c| c.is_ascii_digit() || "-./".contains(c))
            && text.chars().filter(|c| "-./".contains(*c)).count() == 2;
        let value = if text.is_empty() {
            Err("Enter a value".to_owned())
        } else if field.kind == Kind::Date && date_shaped {
            dates::parse(text)
                .map(|serial| Value::exact(Decimal::from(serial)))
                .ok_or_else(|| "Invalid date".to_owned())
        } else {
            engine::evaluate_with(text, context, &variables)
                .map_err(|e| e.message)
                .and_then(|value| {
                    if field.kind == Kind::Date {
                        date_serial(value).map(|_| value)
                    } else {
                        Ok(value)
                    }
                })
        };
        let name = field.name.to_ascii_lowercase();
        match &value {
            Ok(v) => {
                variables.insert(name, *v);
            }
            Err(_) => failed.push(name),
        }
        inputs.push(value);
    }
    let mut outputs = Vec::with_capacity(formula.outputs.len());
    for output in &formula.outputs {
        let value = Expression::parse(&output.expression)
            .map_err(|e| e.message)
            .and_then(|expression| {
                if expression.names().iter().any(|n| failed.contains(n)) {
                    return Err("Depends on a value that could not be calculated".to_owned());
                }
                expression.eval(&context, &variables).map_err(|e| e.message)
            });
        let name = output.name.to_ascii_lowercase();
        match &value {
            Ok(v) => {
                variables.insert(name, *v);
            }
            Err(_) => failed.push(name),
        }
        outputs.push(value);
    }
    Run { inputs, outputs }
}

fn date_serial(value: Value) -> Result<i64, String> {
    value
        .number
        .fract()
        .is_zero()
        .then(|| value.number.to_i64())
        .flatten()
        .filter(|s| dates::in_range(*s))
        .ok_or_else(|| "Invalid date".to_owned())
}

/// Display text for a result, in the user's number style (decimal mark and
/// grouping). Numbers keep up to ten decimals (or the chosen fixed number);
/// money always shows two and percentages up to two.
pub fn format(
    value: Value,
    format: Format,
    language: Language,
    style: NumberStyle,
) -> Result<String, String> {
    let turkish = language == Language::Turkish;
    let shown = |n: Decimal, decimals: Option<u32>| {
        NumberStyle { decimals, ..style }.show(&n.to_string(), language)
    };
    Ok(match format {
        Format::Number => {
            let n = value
                .number
                .round_dp_with_strategy(10, RoundingStrategy::MidpointNearestEven);
            let text = shown(n, style.decimals);
            if value.approximate {
                format!("≈ {text}")
            } else {
                text
            }
        }
        Format::Money => shown(value.number, Some(2)),
        Format::Percent => {
            let text = shown(
                value
                    .number
                    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
                None,
            );
            if turkish {
                format!("%{text}")
            } else {
                format!("{text}%")
            }
        }
        Format::Date => {
            let serial = date_serial(value)?;
            format!(
                "{}  ·  {}",
                dates::format(serial, turkish),
                language.text(dates::weekday_name(serial))
            )
        }
        Format::Weekday => language.text(dates::weekday_name(date_serial(value)?)),
    })
}

#[derive(Serialize, Deserialize)]
struct File {
    format: String,
    version: u32,
    formulas: Vec<Formula>,
}

pub fn to_json(formulas: &[Formula]) -> String {
    serde_json::to_string_pretty(&File {
        format: FORMAT_NAME.into(),
        version: 1,
        formulas: formulas.to_vec(),
    })
    .unwrap_or_default()
}

/// Parse and validate a shared formula file; every formula must be valid.
pub fn from_json(text: &str) -> Result<Vec<Formula>, String> {
    let file: File =
        serde_json::from_str(text).map_err(|_| "This is not a RustCalc formula file".to_owned())?;
    if file.format != FORMAT_NAME {
        return Err("This is not a RustCalc formula file".into());
    }
    if file.version != 1 {
        return Err("Unsupported formula file version".into());
    }
    if file.formulas.len() > MAX_FORMULAS {
        return Err("A formula file can contain at most 200 formulas".into());
    }
    for formula in &file.formulas {
        validate(formula)?;
    }
    Ok(file.formulas)
}

/// Append formulas with fresh ids, skipping ones identical to an existing
/// formula. Returns how many were added; nothing changes on error.
pub fn merge(
    existing: &mut Vec<Formula>,
    incoming: Vec<Formula>,
    seed: u64,
) -> Result<usize, String> {
    let same = |a: &Formula, b: &Formula| {
        Formula {
            id: String::new(),
            ..a.clone()
        } == Formula {
            id: String::new(),
            ..b.clone()
        }
    };
    let mut merged = existing.clone();
    let mut added = 0;
    for (i, mut formula) in incoming.into_iter().enumerate() {
        if merged.iter().any(|f| same(f, &formula)) {
            continue;
        }
        formula.id = new_id(&merged, seed + i as u64);
        merged.push(formula);
        added += 1;
    }
    if merged.len() > MAX_FORMULAS {
        return Err("You can keep at most 200 custom formulas".into());
    }
    *existing = merged;
    Ok(added)
}

/// A unique custom id derived from `seed` (normally a timestamp).
pub fn new_id(existing: &[Formula], seed: u64) -> String {
    let mut n = seed;
    loop {
        let id = format!("custom-{n}");
        if !existing.iter().any(|f| f.id == id) && !is_builtin(&id) {
            return id;
        }
        n += 1;
    }
}

pub fn read_file(path: &std::path::Path) -> Result<Vec<Formula>, String> {
    use std::io::Read;
    let file =
        std::fs::File::open(path).map_err(|_| "Could not open the formula file.".to_owned())?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the formula file.".to_owned())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("The formula file exceeds the 1 MiB limit.".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    from_json(
        std::str::from_utf8(bytes).map_err(|_| "This is not a RustCalc formula file".to_owned())?,
    )
}

/// Atomically write a shared formula file.
pub fn write_file(path: &std::path::Path, formulas: &[Formula]) -> Result<(), String> {
    crate::history::write_file(path, to_json(formulas).as_bytes())
        .map_err(|_| "Could not save the formula file. The original file was kept.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            today: dates::from_ymd(2026, 10, 21),
            ..Default::default()
        }
    }
    fn shown(formula: &Formula, values: &[&str], language: Language) -> Vec<String> {
        let values: Vec<String> = values.iter().map(|s| s.to_string()).collect();
        let run = run(formula, &values, context());
        run.outputs
            .iter()
            .zip(&formula.outputs)
            .map(|(value, output)| match value {
                Ok(v) => format(*v, output.format, language, NumberStyle::default())
                    .unwrap_or_else(|e| e),
                Err(e) => format!("error: {e}"),
            })
            .collect()
    }
    fn builtin(id: &str) -> &'static Formula {
        builtins().iter().find(|f| f.id == id).unwrap()
    }

    #[test]
    fn every_builtin_template_is_valid_and_computes_its_defaults() {
        assert!(builtins().len() >= 20);
        let mut ids = std::collections::BTreeSet::new();
        for formula in builtins() {
            assert!(ids.insert(&formula.id), "duplicate id {}", formula.id);
            validate(formula).unwrap_or_else(|e| panic!("{}: {e}", formula.id));
            let run = run(formula, &formula.defaults(), context());
            for (value, field) in run.inputs.iter().zip(&formula.inputs) {
                assert!(value.is_ok(), "{} {}: {value:?}", formula.id, field.name);
            }
            for (value, output) in run.outputs.iter().zip(&formula.outputs) {
                let value = value
                    .as_ref()
                    .unwrap_or_else(|e| panic!("{} {}: {e}", formula.id, output.name));
                format(
                    *value,
                    output.format,
                    Language::English,
                    NumberStyle::default(),
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn finance_templates_match_reference_values() {
        // 100 000 at 3.5% over 10 years: 988.86 per month (standard annuity).
        assert_eq!(
            shown(builtin("loan"), &["100000", "3.5", "10"], Language::English),
            ["120", "988.86", "118\u{A0}663.04", "18\u{A0}663.04"]
        );
        assert_eq!(
            shown(builtin("loan"), &["12000", "0", "1"], Language::Turkish),
            ["12", "1\u{A0}000,00", "12\u{A0}000,00", "0,00"]
        );
        assert_eq!(
            shown(builtin("vat_remove"), &["1200", "20"], Language::English),
            ["1\u{A0}000.00", "200.00"]
        );
        assert_eq!(
            shown(
                builtin("compound"),
                &["10000", "5", "10", "1"],
                Language::English
            ),
            ["16\u{A0}288.95", "6\u{A0}288.95", "62.89%"]
        );
        assert_eq!(
            shown(builtin("change"), &["80", "100"], Language::Turkish),
            ["20", "%25"]
        );
    }

    #[test]
    fn thousands_separators_are_recognized() {
        for grouped in ["250.000", "1,500", "12.345.678", " 999.000 "] {
            assert!(looks_grouped(grouped), "{grouped}");
        }
        for plain in [
            "250", "2.5", "0.125", "1500", "12.34", "1.5000", "abc", "1.000,5",
        ] {
            assert!(!looks_grouped(plain), "{plain}");
        }
    }

    #[test]
    fn templates_round_like_people_expect() {
        // 179.9 km at 60 km/h is 179.9 minutes: 3 h 0 min, never 2 h 60 min.
        assert_eq!(
            shown(builtin("travel"), &["179.9", "60"], Language::English),
            ["3", "0"]
        );
        assert_eq!(
            shown(builtin("travel"), &["179.55", "90"], Language::English),
            ["2", "0"]
        );
        // A BMI of exactly 22.25 rounds half up.
        assert_eq!(
            shown(builtin("bmi"), &["89", "200"], Language::English)[0],
            "22.3"
        );
        // Small unit prices keep enough digits to compare.
        assert_eq!(
            shown(
                builtin("unit_price"),
                &["2.49", "750", "3.29", "1000"],
                Language::English
            )[..2],
            ["0.00332", "0.00329"]
        );
    }

    #[test]
    fn date_templates_use_calendar_rules() {
        assert_eq!(
            shown(
                builtin("between"),
                &["2026-10-21", "31.12.2026"],
                Language::English
            ),
            ["71", "10.14", "51", "2"]
        );
        assert_eq!(
            shown(builtin("add_days"), &["today()", "-21"], Language::Turkish),
            ["30.09.2026  ·  Çarşamba", "Çarşamba"]
        );
        assert_eq!(
            shown(
                builtin("add_workdays"),
                &["2026-10-23", "1"],
                Language::English
            ),
            ["2026-10-26  ·  Monday", "Monday", "3"]
        );
        assert_eq!(
            shown(
                builtin("age"),
                &["1990-05-15", "2026-10-21"],
                Language::English
            ),
            ["36", "5", "13\u{A0}308", "2027-05-15  ·  Saturday", "206"]
        );
    }

    #[test]
    fn invalid_inputs_are_reported_where_they_occur() {
        let formula = builtin("add_days");
        let values = ["2026-02-30".to_string(), "".to_string()];
        let run = run(formula, &values, context());
        // A date-shaped entry is never reinterpreted as subtraction.
        assert_eq!(run.inputs[0], Err("Invalid date".to_owned()));
        assert_eq!(run.inputs[1], Err("Enter a value".to_owned()));
        assert_eq!(
            run.outputs[0],
            Err("Depends on a value that could not be calculated".to_owned())
        );
        // Number inputs accept expressions and earlier inputs.
        let loan = builtin("loan");
        let values = ["50000 * 2".into(), "7 / 2".into(), "rate * 2 + 3".into()];
        assert_eq!(run_text(loan, &values), "988.86");
        // A date input may be an expression too, but must be a valid date.
        let bad = ["today() + 0,5".to_string(), "1".to_string()];
        assert_eq!(
            super::run(formula, &bad, context()).inputs[0],
            Err("Invalid date".into())
        );
    }
    fn run_text(formula: &Formula, values: &[String]) -> String {
        let run = run(formula, values, context());
        format(
            *run.outputs[1].as_ref().unwrap(),
            Format::Money,
            Language::English,
            NumberStyle::default(),
        )
        .unwrap()
    }

    #[test]
    fn validation_rejects_bad_names_and_forward_references() {
        let mut formula = Formula::blank(Language::English);
        formula.name = "Test".into();
        assert_eq!(validate(&formula), Ok(()));
        let check = |edit: fn(&mut Formula)| {
            let mut f = formula.clone();
            edit(&mut f);
            validate(&f).unwrap_err()
        };
        assert_eq!(
            check(|f| f.name = " ".into()),
            "Enter a name for the formula"
        );
        assert_eq!(check(|f| f.outputs.clear()), "Add at least one result");
        assert_eq!(
            check(|f| f.inputs[0].name = "2a".into()),
            "Invalid variable name: 2a"
        );
        assert_eq!(
            check(|f| f.inputs[0].name = "sqrt".into()),
            "Reserved name: sqrt"
        );
        assert_eq!(
            check(|f| f.outputs[0].name = "A".into()),
            "Duplicate name: A"
        );
        assert_eq!(
            check(|f| f.outputs[0].expression = "b + 1".into()),
            "Unknown name: b"
        );
        assert_eq!(
            check(|f| f.outputs[0].expression = "1 +".into()),
            "Expected a number, constant or function"
        );
        assert_eq!(
            check(|f| {
                f.outputs.insert(
                    0,
                    Output {
                        name: "first".into(),
                        expression: "result + 1".into(),
                        ..Default::default()
                    },
                )
            }),
            "Unknown name: result"
        );
        assert_eq!(
            check(|f| f.description = "x".repeat(501)),
            "Text is too long"
        );
        assert_eq!(
            check(|f| f.inputs = vec![Field::default(); 21]),
            "Too many inputs or results (maximum 20 each)"
        );
    }

    #[test]
    fn files_round_trip_and_merging_skips_duplicates() {
        let mut mine = vec![builtin("bmi").localized_copy(Language::Turkish)];
        mine[0].id = new_id(&mine, 7);
        assert_eq!(mine[0].id, "custom-7");
        assert_eq!(mine[0].name, "Vücut kitle indeksi");
        let text = to_json(&mine);
        let loaded = from_json(&text).unwrap();
        assert_eq!(loaded, mine);
        assert_eq!(merge(&mut mine, loaded, 100), Ok(0));
        let mut other = Formula::blank(Language::English);
        other.name = "Other".into();
        assert_eq!(merge(&mut mine, vec![other.clone(), other], 100), Ok(1));
        assert_eq!(mine[1].id, "custom-100");
        assert_eq!(
            from_json("{}").unwrap_err(),
            "This is not a RustCalc formula file"
        );
        assert_eq!(
            from_json(&text.replace("\"version\": 1", "\"version\": 2")).unwrap_err(),
            "Unsupported formula file version"
        );
        let invalid = text.replace("\"bmi\"", "\"sqrt\"");
        assert_eq!(from_json(&invalid).unwrap_err(), "Reserved name: sqrt");
        let many = vec![Formula::blank(Language::English); MAX_FORMULAS + 1];
        let mut full = Vec::new();
        let distinct: Vec<Formula> = many
            .into_iter()
            .enumerate()
            .map(|(i, mut f)| {
                f.name = format!("F{i}");
                f
            })
            .collect();
        assert!(merge(&mut full, distinct, 1).is_err());
        assert!(full.is_empty(), "a rejected merge changes nothing");
    }

    #[test]
    fn builtin_text_is_translated_into_turkish() {
        let tr = |s: &str| Language::Turkish.text(s);
        for formula in builtins() {
            let mut texts = vec![&formula.name, &formula.category, &formula.description];
            texts.extend(formula.inputs.iter().map(|f| &f.label));
            texts.extend(formula.outputs.iter().map(|o| &o.label));
            for text in texts {
                assert_ne!(
                    &tr(text),
                    text,
                    "{}: missing Turkish for {text:?}",
                    formula.id
                );
            }
            for output in &formula.outputs {
                if !output.unit.is_empty() && !["L", "kWh", "kg"].contains(&output.unit.as_str()) {
                    assert_ne!(tr(&output.unit), output.unit, "unit {}", output.unit);
                }
            }
        }
        for format in Format::ALL {
            assert_ne!(tr(format.title()), format.title());
        }
        for day in dates::WEEKDAYS {
            assert_ne!(tr(day), day);
        }
    }

    #[test]
    fn the_documented_file_example_imports() {
        let guide = include_str!("../docs/FORMULAS.md");
        let start = guide.find("```json").expect("example") + "```json".len();
        let end = start + guide[start..].find("```").unwrap();
        let formulas = from_json(&guide[start..end]).unwrap();
        assert_eq!(formulas[0].name, "Room paint");
        let run = run(&formulas[0], &formulas[0].defaults(), context());
        assert_eq!(run.outputs[0].as_ref().unwrap().text(), "10.4");
    }

    #[test]
    fn money_grouping_and_percent_follow_the_language() {
        let v = |s: &str| Value::exact(s.parse().unwrap());
        assert_eq!(
            format(
                v("1234567.891"),
                Format::Money,
                Language::English,
                NumberStyle::default()
            )
            .unwrap(),
            "1\u{A0}234\u{A0}567.89"
        );
        assert_eq!(
            format(
                v("-1234.5"),
                Format::Money,
                Language::Turkish,
                NumberStyle::default()
            )
            .unwrap(),
            "-1\u{A0}234,50"
        );
        assert_eq!(
            format(
                v("0.125"),
                Format::Money,
                Language::English,
                NumberStyle::default()
            )
            .unwrap(),
            "0.13"
        );
        assert_eq!(
            format(
                v("999"),
                Format::Money,
                Language::English,
                NumberStyle::default()
            )
            .unwrap(),
            "999.00"
        );
        assert_eq!(
            format(
                v("12.345"),
                Format::Percent,
                Language::English,
                NumberStyle::default()
            )
            .unwrap(),
            "12.35%"
        );
        assert_eq!(
            format(
                v("2.5"),
                Format::Number,
                Language::Turkish,
                NumberStyle::default()
            )
            .unwrap(),
            "2,5"
        );
        assert_eq!(
            format(
                Value {
                    number: "0.33333333333333".parse().unwrap(),
                    approximate: true
                },
                Format::Number,
                Language::English,
                NumberStyle::default()
            )
            .unwrap(),
            "≈ 0.3333333333"
        );
        assert_eq!(
            format(
                v("1.5"),
                Format::Date,
                Language::English,
                NumberStyle::default()
            )
            .unwrap_err(),
            "Invalid date"
        );
    }
}
