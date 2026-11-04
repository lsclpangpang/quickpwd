//! 非敏感设置：%USERPROFILE%\.quickpwd\settings.json

use crate::store::app_dir;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
    /// 全局热键，格式如 "Alt+Space"、"Control+Shift+KeyK"
    #[serde(default)]
    pub hotkey: Option<String>,
}

fn path() -> std::path::PathBuf {
    app_dir().join("settings.json")
}

pub fn load() -> Settings {
    fs::read(path())
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> Result<(), String> {
    fs::create_dir_all(app_dir()).map_err(|e| format!("创建数据目录失败: {e}"))?;
    let raw = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    fs::write(path(), raw).map_err(|e| format!("保存设置失败: {e}"))
}
