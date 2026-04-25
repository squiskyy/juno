use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "right" => Self::Right,
            "middle" => Self::Middle,
            _ => Self::Left,
        }
    }
}

pub fn capture_screenshot() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows::Win32::Foundation::*;
            use windows::Win32::Graphics::Gdi::*;
            use windows::Win32::UI::WindowsAndMessaging::*;

            let hdc_screen = GetDC(HWND(0));
            if hdc_screen.is_invalid() {
                return Err("Could not get screen DC".into());
            }
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                let _ = ReleaseDC(HWND(0), hdc_screen);
                return Err("Could not create compatible DC".into());
            }
            let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
            if hbm.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(HWND(0), hdc_screen);
                return Err("Could not create bitmap".into());
            }
            let old = SelectObject(hdc_mem, hbm);
            let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);
            let _ = SelectObject(hdc_mem, old);

            let bwidth = width as u32;
            let bheight = height as u32;
            let row_size = ((bwidth * 3 + 3) / 4) * 4;
            let size = row_size * bheight;
            let file_size = size + 54;

            let mut buf: Vec<u8> = vec![0; size as usize];
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: height,
                    biPlanes: 1,
                    biBitCount: 24,
                    biCompression: BI_RGB.0,
                    biSizeImage: size,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD::default(); 1],
            };
            let _ = GetDIBits(hdc_mem, hbm, 0, bheight, Some(buf.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
            let _ = DeleteObject(hbm.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(HWND(0), hdc_screen);

            let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let mut path = std::env::temp_dir();
            path.push(format!("juno_screenshot_{}.bmp", ts));

            use std::io::Write;
            let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;

            let header: [u8; 54] = [
                0x42, 0x4D,
                (file_size & 0xFF) as u8, ((file_size >> 8) & 0xFF) as u8,
                ((file_size >> 16) & 0xFF) as u8, ((file_size >> 24) & 0xFF) as u8,
                0, 0, 0, 0,
                54, 0, 0, 0,
                40, 0, 0, 0,
                (bwidth & 0xFF) as u8, ((bwidth >> 8) & 0xFF) as u8,
                ((bwidth >> 16) & 0xFF) as u8, ((bwidth >> 24) & 0xFF) as u8,
                (bheight & 0xFF) as u8, ((bheight >> 8) & 0xFF) as u8,
                ((bheight >> 16) & 0xFF) as u8, ((bheight >> 24) & 0xFF) as u8,
                1, 0,
                24, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
            ];
            file.write_all(&header).map_err(|e| e.to_string())?;
            for row in (0..height).rev() {
                let start = (row as u32 * row_size) as usize;
                let end = ((row as u32 + 1) * row_size) as usize;
                file.write_all(&buf[start..end]).map_err(|e| e.to_string())?;
            }
            Ok(path)
        }
    }
    #[cfg(not(target_os = "windows"))]
    { Err("Screenshot only available on Windows".into()) }
}

pub fn click(x: i32, y: i32, button: MouseButton) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            let (down, up) = match button {
                MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
                MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
                MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            };
            windows::Win32::Foundation::SetCursorPos(x, y).map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            let mut input_down = INPUT::default();
            input_down.r#type = INPUT_MOUSE;
            input_down.Anonymous.mi = MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: down, time: 0, dwExtraInfo: 0 };
            let mut input_up = INPUT::default();
            input_up.r#type = INPUT_MOUSE;
            input_up.Anonymous.mi = MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: up, time: 0, dwExtraInfo: 0 };
            let _ = SendInput(&[input_down], core::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = SendInput(&[input_up], core::mem::size_of::<INPUT>() as i32);
            Ok(())
        }
    }
    #[cfg(not(target_os = "windows"))]
    { Err("Mouse control only available on Windows".into()) }
}

fn make_key_input(vk: u16, scan: u16, flags: u32) -> windows::Win32::UI::WindowsAndMessaging::INPUT {
    let mut input = windows::Win32::UI::WindowsAndMessaging::INPUT::default();
    input.r#type = windows::Win32::UI::WindowsAndMessaging::INPUT_KEYBOARD;
    input.Anonymous.ki = windows::Win32::UI::WindowsAndMessaging::KEYBDINPUT {
        wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0,
    };
    input
}

pub fn type_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        for ch in text.chars() {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::*;
                if ch.is_ascii() {
                    let vk = ch.to_ascii_uppercase() as u16;
                    let shift = ch.is_ascii_uppercase()
                        || matches!(ch, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~');
                    let mut inputs: Vec<INPUT> = Vec::new();
                    if shift { inputs.push(make_key_input(0x10, 0, 0)); }
                    inputs.push(make_key_input(vk, 0, 0));
                    inputs.push(make_key_input(vk, 0, KEYEVENTF_KEYUP.0));
                    if shift { inputs.push(make_key_input(0x10, 0, KEYEVENTF_KEYUP.0)); }
                    let _ = SendInput(&inputs, core::mem::size_of::<INPUT>() as i32);
                } else {
                    let scan = ch as u16;
                    let down = make_key_input(0, scan, KEYEVENTF_UNICODE.0);
                    let up = make_key_input(0, scan, KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0);
                    let _ = SendInput(&[down], core::mem::size_of::<INPUT>() as i32);
                    let _ = SendInput(&[up], core::mem::size_of::<INPUT>() as i32);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    { Err("Typing only available on Windows".into()) }
}
