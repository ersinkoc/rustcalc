//! Small command surface for installers and unattended update checks. All CLI
//! output stays English; GUI language is selected independently.
use std::{io::Write, process::ExitCode};

pub const HELP: &str = "RustCalc — a native desktop calculator\n\nUSAGE:\n  rustcalc [--lang en|tr] [--compact] [--light]\n  rustcalc --version\n  rustcalc update --check\n  rustcalc update\n  rustcalc update --rollback\n\nOPTIONS:\n  --lang en|tr   Start in English or Turkish (saved for the next launch)\n  --compact      Open a 760 x 760 window\n  --light        Start with the light appearance\n  --help, -h     Show this help\n  --version, -V  Print the installed version\n  --formulas     Open the formulas workspace\n\nUpdates are opt-in and use verified GitHub Releases from ersinkoc/rustcalc.\n";

pub fn handle(args: &[String], output: &mut impl Write) -> Result<Option<ExitCode>, String> {
    let first = args.first().map(String::as_str);
    match first {
        Some("--version" | "-V") if args.len() == 1 => {
            writeln!(output, "rustcalc {}", env!("CARGO_PKG_VERSION"))
                .map_err(|e| e.to_string())?;
            return Ok(Some(ExitCode::SUCCESS));
        }
        Some("--help" | "-h") if args.len() == 1 => {
            write!(output, "{HELP}").map_err(|e| e.to_string())?;
            return Ok(Some(ExitCode::SUCCESS));
        }
        Some("update") if args.get(1).map(String::as_str) == Some("--rollback") => {
            if args.len() != 2 {
                return Err("Usage: rustcalc update [--check | --rollback]".into());
            }
            let version = crate::updater::rollback()?;
            writeln!(
                output,
                "Restored RustCalc {version}. Run rustcalc update --rollback again to undo."
            )
            .map_err(|e| e.to_string())?;
            return Ok(Some(ExitCode::SUCCESS));
        }
        Some("update") => {
            if args.len() > 2 || (args.len() == 2 && args[1] != "--check") {
                return Err("Usage: rustcalc update [--check | --rollback]".into());
            }
            writeln!(
                output,
                "Checking https://github.com/ersinkoc/rustcalc/releases …"
            )
            .map_err(|e| e.to_string())?;
            match crate::updater::check()? {
                None => {
                    writeln!(output, "RustCalc is up to date.").map_err(|e| e.to_string())?;
                }
                Some(plan) => {
                    writeln!(
                        output,
                        "Update available: {} -> {}",
                        env!("CARGO_PKG_VERSION"),
                        plan.version
                    )
                    .map_err(|e| e.to_string())?;
                    if args.len() == 1 {
                        crate::updater::install(&plan)?;
                        writeln!(
                            output,
                            "Update installed. Restart RustCalc to use the new version."
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
            return Ok(Some(ExitCode::SUCCESS));
        }
        _ => {}
    }
    validate_gui_args(args)?;
    Ok(None)
}

pub fn validate_gui_args(args: &[String]) -> Result<(), String> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lang" => {
                i += 1;
                crate::i18n::Language::parse(args.get(i).ok_or("--lang requires en or tr")?)?;
            }
            "--screenshot" => {
                i += 1;
                if args
                    .get(i)
                    .is_none_or(|s| s.is_empty() || s.starts_with("--"))
                {
                    return Err("--screenshot requires a PNG file path".into());
                }
            }
            "--compact" | "--light" | "--demo" | "--converter" | "--programmer" | "--graph"
            | "--formulas" => {}
            value => return Err(format!("Unknown argument: {value}. Run rustcalc --help.")),
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn version_and_help_work_without_opening_the_gui_or_network() {
        let mut output = Vec::new();
        assert!(
            handle(&args(&["--version"]), &mut output)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("rustcalc {}\n", env!("CARGO_PKG_VERSION"))
        );
        let mut output = Vec::new();
        assert!(handle(&args(&["--help"]), &mut output).unwrap().is_some());
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("update --check")
        );
    }
    #[test]
    fn invalid_and_incomplete_arguments_are_rejected() {
        for values in [
            &["--lang"][..],
            &["--lang", "de"],
            &["--unknown"],
            &["--screenshot"],
            &["update", "--force"],
            &["--version", "extra"],
        ] {
            assert!(handle(&args(values), &mut Vec::new()).is_err());
        }
        assert!(
            handle(&args(&["--lang", "tr", "--compact"]), &mut Vec::new())
                .unwrap()
                .is_none()
        );
    }
}
