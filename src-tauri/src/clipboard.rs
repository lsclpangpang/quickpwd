//! 剪贴板：复制时排除出 Win+V 历史与云同步，N 秒后若剪贴板仍是我们写入的内容则清空。

use std::thread;
use std::time::Duration;

pub fn copy_secret(text: &str, clear_after: Duration) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("无法打开剪贴板: {e}"))?;
    #[cfg(windows)]
    {
        use arboard::SetExtWindows;
        cb.set()
            .exclude_from_cloud()
            .exclude_from_history()
            .text(text.to_owned())
            .map_err(|e| format!("写入剪贴板失败: {e}"))?;
    }
    #[cfg(not(windows))]
    cb.set_text(text.to_owned()).map_err(|e| format!("写入剪贴板失败: {e}"))?;

    let seq = sequence_number();
    thread::spawn(move || {
        thread::sleep(clear_after);
        // 序号未变说明期间用户没复制过别的内容，才清空
        if sequence_number() == seq {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.clear();
            }
        }
    });
    Ok(())
}

#[cfg(windows)]
fn sequence_number() -> u32 {
    unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

#[cfg(not(windows))]
fn sequence_number() -> u32 {
    0
}

