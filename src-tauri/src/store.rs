//! 密码库存储：%USERPROFILE%\.quickpwd\store.dat
//! 内容为 JSON，经 Windows DPAPI（CryptProtectData）加密，只有当前 Windows 账户能解开。
//! 首次启动会自动导入旧版明文 store.json，并把旧文件改名为 store.json.imported。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// 标题 -> 要复制的内容
pub type Store = BTreeMap<String, String>;

pub fn app_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".quickpwd")
}

fn dat_path() -> PathBuf {
    app_dir().join("store.dat")
}

fn legacy_path() -> PathBuf {
    app_dir().join("store.json")
}

pub fn load() -> Result<Store, String> {
    let dat = dat_path();
    if dat.exists() {
        let blob = fs::read(&dat).map_err(|e| format!("读取 {} 失败: {e}", dat.display()))?;
        let plain = dpapi::unprotect(&blob)?;
        return serde_json::from_slice(&plain).map_err(|e| format!("数据文件格式错误: {e}"));
    }
    let legacy = legacy_path();
    if legacy.exists() {
        let raw = fs::read(&legacy).map_err(|e| format!("读取旧版 store.json 失败: {e}"))?;
        let store = parse_legacy(&raw)?;
        save(&store)?;
        let _ = fs::rename(&legacy, app_dir().join("store.json.imported"));
        return Ok(store);
    }
    Ok(Store::new())
}

/// 旧版格式 {关键字: {username, password, note}}，只取 password 作为内容
fn parse_legacy(raw: &[u8]) -> Result<Store, String> {
    let map: BTreeMap<String, serde_json::Value> =
        serde_json::from_slice(raw).map_err(|e| format!("旧版 store.json 格式错误: {e}"))?;
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            let content = match &v {
                serde_json::Value::String(s) => s.clone(),
                _ => v.get("password").and_then(|p| p.as_str()).unwrap_or_default().to_string(),
            };
            (k, content)
        })
        .collect())
}

pub fn save(store: &Store) -> Result<(), String> {
    fs::create_dir_all(app_dir()).map_err(|e| format!("创建数据目录失败: {e}"))?;
    let plain = serde_json::to_vec(store).map_err(|e| e.to_string())?;
    let blob = dpapi::protect(&plain)?;
    // 先写临时文件再替换，避免写到一半断电导致数据损坏
    let tmp = app_dir().join("store.dat.tmp");
    fs::write(&tmp, blob).map_err(|e| format!("写入失败: {e}"))?;
    fs::rename(&tmp, dat_path()).map_err(|e| format!("替换数据文件失败: {e}"))
}

#[cfg(windows)]
mod dpapi {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    const ENTROPY: &[u8] = b"QuickPwd-store-v2";

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as _);
        v
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(data);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        unsafe {
            if CryptProtectData(&input, null(), &entropy, null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
                return Err(format!("加密失败: {}", std::io::Error::last_os_error()));
            }
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(data);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        unsafe {
            if CryptUnprotectData(&input, null_mut(), &entropy, null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
                return Err(format!(
                    "解密失败（数据文件可能来自其他电脑或其他 Windows 账户）: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(take(out))
        }
    }
}

// 非 Windows 平台仅用于开发编译，不加密
#[cfg(not(windows))]
mod dpapi {
    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
}
