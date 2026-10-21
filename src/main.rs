#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
use std::sync::atomic::{AtomicBool, Ordering};
static RESTART: AtomicBool = AtomicBool::new(false);

/// Lets a GUI-subsystem build print to the terminal it was started from.
fn attach_console() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn main() -> std::process::ExitCode {
    // Remember the program's path before an update can replace the file.
    let executable = rustcalc::updater::running_executable();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|s| matches!(s.as_str(), "--version" | "-V" | "--help" | "-h" | "update"))
    {
        attach_console();
    }
    match rustcalc::cli::handle(&args, &mut std::io::stdout()) {
        Ok(Some(code)) => return code,
        Err(error) => {
            // A mistyped option must not fail silently from a terminal.
            attach_console();
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
        Ok(None) => {}
    }
    if let Err(error) = run_gui() {
        eprintln!("Could not start RustCalc: {error}");
        return std::process::ExitCode::FAILURE;
    }
    if RESTART.load(Ordering::SeqCst)
        && let Ok(exe) = executable
        && let Err(error) = std::process::Command::new(exe).spawn()
    {
        eprintln!("Could not restart RustCalc: {error}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run_gui() -> eframe::Result {
    let compact = std::env::args().any(|s| s == "--compact");
    let screenshot = std::env::args().any(|s| s == "--screenshot");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RustCalc")
            .with_inner_size(if compact {
                [760.0, 760.0]
            } else {
                [1180.0, 820.0]
            })
            .with_min_inner_size([720.0, 700.0])
            .with_icon(ui::app_icon()),
        persist_window: !screenshot,
        ..Default::default()
    };
    eframe::run_native(
        "RustCalc",
        options,
        Box::new(|cc| Ok(Box::new(ui::RustCalc::new(cc)))),
    )
}
