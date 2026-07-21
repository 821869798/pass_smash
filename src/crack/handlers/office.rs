use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use anyhow::Context;
use office_crypto::{DecryptError, decrypt_from_bytes};
use parking_lot::RwLock;

use super::PasswordHandler;

/// Cache raw file bytes so each password attempt reuses the same buffer.
static FILE_CACHE: LazyLock<RwLock<HashMap<String, Arc<Vec<u8>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct OfficeHandler;

impl OfficeHandler {
    fn bytes_for(&self, path: &Path) -> anyhow::Result<Arc<Vec<u8>>> {
        let key = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned();
        if let Some(v) = FILE_CACHE.read().get(&key).cloned() {
            return Ok(v);
        }
        let raw = fs::read(path)
            .with_context(|| format!("读取 Office 文件失败: {}", path.display()))?;
        let arc = Arc::new(raw);
        FILE_CACHE.write().insert(key, Arc::clone(&arc));
        Ok(arc)
    }
}

impl PasswordHandler for OfficeHandler {
    fn is_encrypted(&self, path: &Path) -> anyhow::Result<bool> {
        let bytes = self.bytes_for(path)?;
        // Plain OOXML is a ZIP package (PK..), not OLE.
        if looks_like_zip(bytes.as_slice()) {
            return Ok(false);
        }
        // Encrypted OOXML / legacy DOC are OLE compound files.
        if looks_like_ole(bytes.as_slice()) {
            return Ok(true);
        }
        // Unknown layout — try empty decrypt; NotEncrypted/InvalidHeader => not encrypted.
        match decrypt_from_bytes(bytes.as_slice().to_vec(), "") {
            Ok(_) => Ok(false),
            Err(DecryptError::NotEncrypted) | Err(DecryptError::InvalidHeader) => Ok(false),
            Err(DecryptError::Unimplemented(msg)) => {
                Err(anyhow::anyhow!("Office 格式暂不支持: {msg}"))
            }
            Err(DecryptError::IoError(e)) => {
                Err(e).with_context(|| format!("检测 Office 加密失败: {}", path.display()))
            }
            Err(_) => Ok(true),
        }
    }

    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool> {
        let bytes = self.bytes_for(path)?;
        // Unencrypted ZIP-based OOXML: treat as success (nothing to crack).
        if looks_like_zip(bytes.as_slice()) {
            return Ok(true);
        }

        match decrypt_from_bytes(bytes.as_slice().to_vec(), password) {
            Ok(plain) => {
                // office-crypto may return Ok even for wrong passwords (garbage bytes).
                // Correct OOXML decrypts to a ZIP package; legacy DOC to OLE.
                Ok(looks_like_zip(&plain) || looks_like_ole(&plain))
            }
            Err(DecryptError::NotEncrypted) => Ok(true),
            Err(DecryptError::InvalidStructure)
            | Err(DecryptError::InvalidHeader)
            | Err(DecryptError::Unknown) => Ok(false),
            Err(DecryptError::Unimplemented(msg)) => {
                Err(anyhow::anyhow!("Office 格式暂不支持: {msg}"))
            }
            Err(DecryptError::IoError(e)) => {
                Err(e).with_context(|| format!("验证 Office 密码时出错: {}", path.display()))
            }
        }
    }
}

fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B
}

fn looks_like_ole(bytes: &[u8]) -> bool {
    // D0 CF 11 E0 A1 B1 1A E1
    bytes.len() >= 8
        && bytes[0] == 0xD0
        && bytes[1] == 0xCF
        && bytes[2] == 0x11
        && bytes[3] == 0xE0
}

#[cfg(test)]
#[allow(dead_code)]
pub fn clear_cache() {
    FILE_CACHE.write().clear();
}
