mod clipboard;
mod store;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const CLEAR_AFTER_SECS: u64 = 30;
const AUTOSTART_ARG: &str = "--autostart";

// 按顺序尝试，注册第一个可用的组合键（Alt+Space 被其他程序占用时依次降级）
const HOTKEY_CANDIDATES: &[(&str, Modifiers)] = &[
    ("Alt + Space", Modifiers::ALT),
    ("Ctrl + Alt + Space", Modifiers::CONTROL.union(Modifiers::ALT)),
    ("Ctrl + Shift + Space", Modifiers::CONTROL.union(Modifiers::SHIFT)),
    ("Shift + Alt + Space", Modifiers::SHIFT.union(Modifiers::ALT)),
    ("Win + Alt + Space", Modifiers::SUPER.union(Modifiers::ALT)),
];

#[derive(Default)]
struct AppState {
    store: Mutex<store::Store>,
    // 数据文件读取失败时禁止写入，防止用空库覆盖原文件
    load_error: Mutex<Option<String>>,
    hotkey: Mutex<Option<String>>,
    appearance: Mutex<String>,
}

#[derive(Deserialize)]
struct EntryInput {
    title: String,
    content: String,
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    hotkey: Option<String>,
    autostart: bool,
    appearance: String,
    clear_after: u64,
    data_dir: String,
    load_error: Option<String>,
}

fn notify_changed(app: &AppHandle) {
    let _ = app.emit("store-changed", ());
}

fn ensure_writable(state: &AppState) -> Result<(), String> {
    match &*state.load_error.lock().unwrap() {
        Some(e) => Err(format!("数据文件读取失败，为防止覆盖已禁止写入。{e}")),
        None => Ok(()),
    }
}

// ---------------- 命令 ----------------

#[tauri::command]
fn list_titles(state: State<AppState>) -> Vec<String> {
    state.store.lock().unwrap().keys().cloned().collect()
}

#[tauri::command]
fn get_content(state: State<AppState>, title: String) -> Result<String, String> {
    state.store.lock().unwrap().get(&title).cloned().ok_or_else(|| format!("条目「{title}」不存在"))
}

#[tauri::command]
fn save_entry(
    app: AppHandle,
    state: State<AppState>,
    original: Option<String>,
    entry: EntryInput,
) -> Result<(), String> {
    ensure_writable(&state)?;
    let title = entry.title.trim().to_string();
    if title.is_empty() {
        return Err("标题不能为空".into());
    }
    if entry.content.is_empty() {
        return Err("内容不能为空".into());
    }
    let mut guard = state.store.lock().unwrap();
    let mut next = guard.clone();
    if original.as_deref() != Some(title.as_str()) && next.contains_key(&title) {
        return Err(format!("标题「{title}」已存在"));
    }
    if let Some(old) = &original {
        next.remove(old);
    }
    next.insert(title, entry.content);
    store::save(&next)?;
    *guard = next;
    drop(guard);
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
fn delete_entry(app: AppHandle, state: State<AppState>, title: String) -> Result<(), String> {
    ensure_writable(&state)?;
    let mut guard = state.store.lock().unwrap();
    let mut next = guard.clone();
    if next.remove(&title).is_none() {
        return Err(format!("条目「{title}」不存在"));
    }
    store::save(&next)?;
    *guard = next;
    drop(guard);
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
fn copy_entry(state: State<AppState>, title: String) -> Result<u64, String> {
    let content = get_content(state, title)?;
    clipboard::copy_secret(&content, Duration::from_secs(CLEAR_AFTER_SECS))?;
    Ok(CLEAR_AFTER_SECS)
}

#[tauri::command]
fn app_info(app: AppHandle, state: State<AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        hotkey: state.hotkey.lock().unwrap().clone(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        appearance: state.appearance.lock().unwrap().clone(),
        clear_after: CLEAR_AFTER_SECS,
        data_dir: store::app_dir().display().to_string(),
        load_error: state.load_error.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let al = app.autolaunch();
    let res = if enabled { al.enable() } else { al.disable() };
    res.map_err(|e| format!("设置开机自启失败: {e}"))?;
    Ok(al.is_enabled().unwrap_or(enabled))
}

#[tauri::command]
fn open_manager(app: AppHandle) {
    hide_launcher_window(&app);
    show_manager(&app);
}

#[tauri::command]
fn hide_launcher(app: AppHandle) {
    hide_launcher_window(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

// ---------------- 窗口控制 ----------------

fn show_manager(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("manager") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn hide_launcher_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.hide();
    }
}

/// 放在鼠标所在显示器的水平居中、靠上 18% 的位置
fn position_launcher(app: &AppHandle, w: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, w.outer_size()) else { return };
    let area = m.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + (area.size.height as f64 * 0.18) as i32;
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

fn toggle_launcher(app: &AppHandle) {
    let Some(w) = app.get_webview_window("launcher") else { return };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    position_launcher(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("launcher-show", ());
}

fn apply_launcher_backdrop(w: &WebviewWindow) -> &'static str {
    #[cfg(windows)]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        if apply_mica(w, Some(true)).is_ok() {
            return "mica";
        }
        if apply_acrylic(w, Some((18, 22, 32, 170))).is_ok() {
            return "acrylic";
        }
    }
    let _ = w;
    "solid"
}

fn register_hotkey(app: &AppHandle) -> Option<String> {
    for (name, mods) in HOTKEY_CANDIDATES {
        if app.global_shortcut().register(Shortcut::new(Some(*mods), Code::Space)).is_ok() {
            return Some((*name).to_string());
        }
    }
    None
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开管理窗口", true, None::<&str>)?;
    let search = MenuItem::with_id(app, "search", "快速搜索", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 QuickPwd", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &search, &quit])?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("缺少应用图标"))
        .tooltip("QuickPwd")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_manager(app),
            "search" => toggle_launcher(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_manager(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：重复启动时唤出已运行实例的管理窗口
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| show_manager(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_launcher(app);
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();

            match store::load() {
                Ok(s) => *state.store.lock().unwrap() = s,
                Err(e) => *state.load_error.lock().unwrap() = Some(e),
            }
            *state.hotkey.lock().unwrap() = register_hotkey(&handle);

            if let Some(w) = app.get_webview_window("launcher") {
                *state.appearance.lock().unwrap() = apply_launcher_backdrop(&w).to_string();
            }
            build_tray(&handle)?;

            let autostarted = std::env::args().any(|a| a == AUTOSTART_ARG);
            let needs_attention = state.load_error.lock().unwrap().is_some() || state.hotkey.lock().unwrap().is_none();
            let empty = state.store.lock().unwrap().is_empty();
            if needs_attention || empty {
                show_manager(&handle);
            } else if !autostarted {
                // 手动启动时直接弹出搜索框，确认热键可用
                toggle_launcher(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // 搜索框失焦即隐藏
            ("launcher", WindowEvent::Focused(false)) => {
                let _ = window.hide();
            }
            // 关闭管理窗口只隐藏，程序常驻托盘
            ("manager", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            list_titles,
            get_content,
            save_entry,
            delete_entry,
            copy_entry,
            app_info,
            set_autostart,
            open_manager,
            hide_launcher,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("QuickPwd 启动失败");
}
