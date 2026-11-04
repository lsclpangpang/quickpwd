//! 低层键盘钩子（WH_KEYBOARD_LL），只在需要时安装：
//! - 录制快捷键：Win+Space、Alt+Space 等组合会在到达窗口前被系统处理，必须在钩子里捕获；
//! - 接管 RegisterHotKey 拿不到的组合（Win 组合、Alt+键）。

use crate::accel::{self, Combo, Mods};
use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const RECORD_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Serialize)]
pub struct Capture {
    /// mods：只按了修饰键；combo：完整组合；invalid：缺少修饰键；reserved：Alt+Tab 等系统组合；cancel / confirm：Esc / Enter
    pub kind: &'static str,
    pub accelerator: String,
}

enum Mode {
    Idle,
    Listen(Combo),
    Record { held: Mods, started: Instant },
}

struct State {
    mode: Mode,
    thread_id: u32,
    swallow_up: Option<u32>,
    app: Option<AppHandle>,
    on_trigger: Option<fn(&AppHandle)>,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State { mode: Mode::Idle, thread_id: 0, swallow_up: None, app: None, on_trigger: None })
    })
}

pub fn init(app: AppHandle, on_trigger: fn(&AppHandle)) {
    let mut s = state().lock().unwrap();
    s.app = Some(app);
    s.on_trigger = Some(on_trigger);
}

/// 用钩子监听这个组合；None 表示不再监听。返回钩子是否安装成功。
pub fn listen(combo: Option<Combo>) -> bool {
    let mut s = state().lock().unwrap();
    s.mode = match combo {
        Some(c) => Mode::Listen(c),
        None => Mode::Idle,
    };
    s.swallow_up = None;
    sync_thread(&mut s)
}

pub fn start_record() -> bool {
    let mut s = state().lock().unwrap();
    s.mode = Mode::Record { held: Mods::default(), started: Instant::now() };
    sync_thread(&mut s)
}

/// 结束录制，回到空闲；调用方随后重新激活当前热键
pub fn stop_record() {
    let mut s = state().lock().unwrap();
    if matches!(s.mode, Mode::Record { .. }) {
        s.mode = Mode::Idle;
        sync_thread(&mut s);
    }
}

#[cfg(windows)]
use imp::sync_thread;

#[cfg(not(windows))]
fn sync_thread(_s: &mut State) -> bool {
    false
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ptr::null_mut;
    use std::sync::mpsc;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
        VK_ESCAPE, VK_F1, VK_F12, VK_F4, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN,
        VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
        LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    /// 需要钩子时启动钩子线程，空闲时停掉，避免常驻全局键盘钩子
    pub(super) fn sync_thread(s: &mut State) -> bool {
        let need = !matches!(s.mode, Mode::Idle);
        if need && s.thread_id == 0 {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || unsafe {
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), null_mut(), 0);
                if hook.is_null() {
                    let _ = tx.send(0);
                    return;
                }
                let _ = tx.send(GetCurrentThreadId());
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
                UnhookWindowsHookEx(hook);
            });
            s.thread_id = rx.recv().unwrap_or(0);
            return s.thread_id != 0;
        }
        if !need && s.thread_id != 0 {
            unsafe { PostThreadMessageW(s.thread_id, WM_QUIT, 0, 0) };
            s.thread_id = 0;
        }
        !need || s.thread_id != 0
    }

    enum Action {
        None,
        Trigger { app: AppHandle, f: fn(&AppHandle), dummy: bool },
        Emit(AppHandle, Capture),
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            // 忽略注入的按键（包括我们自己发的占位键）
            if kb.flags & LLKHF_INJECTED == 0 {
                let msg = wparam as u32;
                let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                // 先在锁内决定动作，释放锁之后再执行，避免回调中重入
                let (swallow, action) = decide(kb.vkCode, down);
                run(action);
                if swallow {
                    return 1;
                }
            }
        }
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }

    fn modifier_of(vk: u32) -> Option<fn(&mut Mods) -> &mut bool> {
        let vk = vk as u16;
        if [VK_CONTROL, VK_LCONTROL, VK_RCONTROL].contains(&vk) {
            Some(|m| &mut m.ctrl)
        } else if [VK_MENU, VK_LMENU, VK_RMENU].contains(&vk) {
            Some(|m| &mut m.alt)
        } else if [VK_SHIFT, VK_LSHIFT, VK_RSHIFT].contains(&vk) {
            Some(|m| &mut m.shift)
        } else if [VK_LWIN, VK_RWIN].contains(&vk) {
            Some(|m| &mut m.win)
        } else {
            None
        }
    }

    fn pressed(vk: u16) -> bool {
        unsafe { (GetAsyncKeyState(vk as i32) as u16) & 0x8000 != 0 }
    }

    fn current_mods() -> Mods {
        Mods {
            ctrl: pressed(VK_CONTROL),
            alt: pressed(VK_MENU),
            shift: pressed(VK_SHIFT),
            win: pressed(VK_LWIN) || pressed(VK_RWIN),
        }
    }

    fn decide(vk: u32, down: bool) -> (bool, Action) {
        let Ok(mut s) = state().try_lock() else { return (false, Action::None) };
        let app = s.app.clone();
        match &mut s.mode {
            Mode::Idle => (false, Action::None),
            Mode::Listen(combo) => {
                let combo = *combo;
                if !down {
                    if s.swallow_up == Some(vk) {
                        s.swallow_up = None;
                        return (true, Action::None);
                    }
                    return (false, Action::None);
                }
                if modifier_of(vk).is_some() || vk != combo.vk || current_mods() != combo.mods {
                    return (false, Action::None);
                }
                s.swallow_up = Some(vk);
                match (app, s.on_trigger) {
                    (Some(app), Some(f)) => {
                        (true, Action::Trigger { app, f, dummy: combo.mods.win || combo.mods.alt })
                    }
                    _ => (true, Action::None),
                }
            }
            Mode::Record { held, started } => {
                let Some(app) = app else { return (true, Action::None) };
                if started.elapsed() > RECORD_TIMEOUT {
                    // 安全阀：录制窗口异常时不让键盘一直被吞
                    s.mode = Mode::Idle;
                    let emit = Action::Emit(app, Capture { kind: "cancel", accelerator: String::new() });
                    return (false, emit);
                }
                if let Some(field) = modifier_of(vk) {
                    *field(held) = down;
                    // 只在按下时推送，松开修饰键不应清掉已录到的组合
                    if !down {
                        return (true, Action::None);
                    }
                    let cap = Capture { kind: "mods", accelerator: accel::format(*held, None) };
                    return (true, Action::Emit(app, cap));
                }
                if !down {
                    return (true, Action::None);
                }
                let mods = *held;
                let vk16 = vk as u16;
                let alt_only = mods == Mods { alt: true, ..Mods::default() };
                let kind = if alt_only && [VK_TAB, VK_F4, VK_ESCAPE].contains(&vk16) {
                    "reserved"
                } else if !mods.any() && vk16 == VK_ESCAPE {
                    "cancel"
                } else if !mods.any() && vk16 == VK_RETURN {
                    "confirm"
                } else if !mods.any() && !(VK_F1..=VK_F12).contains(&vk16) {
                    "invalid"
                } else {
                    "combo"
                };
                let Some(code) = accel::vk_to_code(vk) else { return (true, Action::None) };
                let cap = Capture { kind, accelerator: accel::format(mods, Some(&code)) };
                (true, Action::Emit(app, cap))
            }
        }
    }

    fn run(action: Action) {
        match action {
            Action::None => {}
            Action::Trigger { app, f, dummy } => {
                if dummy {
                    send_dummy_key();
                }
                let a = app.clone();
                let _ = app.run_on_main_thread(move || f(&a));
            }
            Action::Emit(app, cap) => {
                let _ = app.emit("hotkey-capture", cap);
            }
        }
    }

    /// 发送一个未分配的按键（0xE8），防止松开 Win 弹出开始菜单、松开 Alt 激活菜单栏
    fn send_dummy_key() {
        let key = |flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: 0xE8, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        };
        let inputs = [key(0), key(KEYEVENTF_KEYUP)];
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }
}
