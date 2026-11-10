mod app;
mod calculator;
mod formulas;
mod screenshot;
mod settings;
mod shortcuts;
mod storage;
mod theme;
mod workspaces;

pub fn run(args: Vec<String>) -> gpui::Result<()> {
    app::run(args)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(windows)]
fn system_high_contrast() -> bool {
    use windows_sys::Win32::UI::{
        Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
        WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SystemParametersInfoW},
    };
    let mut settings = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    // The structure is sized and writable as required by SystemParametersInfoW.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            settings.cbSize,
            (&mut settings as *mut HIGHCONTRASTW).cast(),
            0,
        )
    };
    ok != 0 && settings.dwFlags & HCF_HIGHCONTRASTON != 0
}

#[cfg(not(windows))]
fn system_high_contrast() -> bool {
    false
}
