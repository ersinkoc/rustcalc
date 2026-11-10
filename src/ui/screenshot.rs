use super::app::Calc;
use gpui::{Context, Window};

impl Calc {
    pub fn capture_screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.screenshot.take() {
            if let Err(error) = save(window, &path) {
                eprintln!("Could not save screenshot: {error}");
                crate::SCREENSHOT_FAILED.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            cx.quit();
        }
    }
}

#[cfg(not(windows))]
fn save(_: &Window, _: &std::path::Path) -> Result<(), String> {
    Err("Screenshot export is available on Windows.".into())
}

#[cfg(windows)]
fn save(window: &Window, path: &std::path::Path) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::RECT, Graphics::Gdi::*, Storage::Xps::PrintWindow,
        UI::WindowsAndMessaging::GetClientRect,
    };
    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Missing Win32 window handle.".into());
    };
    let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    // GDI owns the DIB buffer until the bitmap is deleted. Copy RGB before
    // restoring the DC and releasing the handles.
    let (width, height, pixels) = unsafe {
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect) == 0 || rect.right <= 0 || rect.bottom <= 0 {
            return Err("Could not read the window size.".into());
        }
        let (width, height) = (rect.right, rect.bottom);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        if dc.is_null() {
            return Err("Could not create a screenshot context.".into());
        }
        let mut data = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            dc,
            &info,
            DIB_RGB_COLORS,
            &mut data,
            std::ptr::null_mut(),
            0,
        );
        if bitmap.is_null() || data.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            DeleteDC(dc);
            return Err("Could not allocate a screenshot bitmap.".into());
        }
        let previous = SelectObject(dc, bitmap);
        // PW_CLIENTONLY | PW_RENDERFULLCONTENT includes DirectComposition.
        let mut captured = PrintWindow(hwnd, dc, 3) != 0;
        let read_pixels =
            || std::slice::from_raw_parts(data.cast::<[u8; 4]>(), width as usize * height as usize);
        GdiFlush();
        if !captured || !read_pixels().iter().any(|bgra| bgra[..3] != [0, 0, 0]) {
            let source = GetDC(hwnd);
            captured =
                !source.is_null() && BitBlt(dc, 0, 0, width, height, source, 0, 0, SRCCOPY) != 0;
            if !source.is_null() {
                ReleaseDC(hwnd, source);
            }
        }
        GdiFlush();
        let pixels: Vec<_> = read_pixels()
            .iter()
            .flat_map(|bgra| [bgra[2], bgra[1], bgra[0]])
            .collect();
        SelectObject(dc, previous);
        DeleteObject(bitmap);
        DeleteDC(dc);
        if !captured || !pixels.iter().any(|&value| value != 0) {
            return Err("Windows did not return a rendered frame.".into());
        }
        (width as u32, height as u32, pixels)
    };
    image::save_buffer(path, &pixels, width, height, image::ColorType::Rgb8)
        .map_err(|e| e.to_string())
}
