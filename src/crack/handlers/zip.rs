use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, bail};
use zip::ZipArchive;

use super::PasswordHandler;

pub struct ZipHandler;

impl PasswordHandler for ZipHandler {
    fn is_encrypted(&self, path: &Path) -> anyhow::Result<bool> {
        let file = File::open(path).with_context(|| format!("打开文件失败: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut archive =
            ZipArchive::new(reader).with_context(|| format!("解析 ZIP 失败: {}", path.display()))?;

        for i in 0..archive.len() {
            // by_index fails on encrypted entries without a password; use raw metadata.
            let entry = archive
                .by_index_raw(i)
                .with_context(|| format!("读取 ZIP 条目 {i} 失败"))?;
            if entry.encrypted() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool> {
        let file = File::open(path).with_context(|| format!("打开文件失败: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut archive =
            ZipArchive::new(reader).with_context(|| format!("解析 ZIP 失败: {}", path.display()))?;

        if archive.len() == 0 {
            bail!("ZIP 为空");
        }

        // Prefer an encrypted non-directory entry; otherwise first non-dir file.
        let mut target_index = None;
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i)?;
            let is_dir = entry.is_dir();
            let encrypted = entry.encrypted();
            drop(entry);
            if !is_dir && encrypted {
                target_index = Some(i);
                break;
            }
            if target_index.is_none() && !is_dir {
                target_index = Some(i);
            }
        }

        let Some(idx) = target_index else {
            return Ok(true);
        };

        // ZipCrypto only has a weak traditional password check. Always fully
        // decompress + verify CRC so wrong passwords are not accepted.
        match archive.by_index_decrypt(idx, password.as_bytes()) {
            Ok(mut entry) => match read_and_verify(&mut entry) {
                Ok(true) => Ok(true),
                Ok(false) => Ok(false),
                Err(e) => {
                    let msg = e.to_string().to_ascii_lowercase();
                    if msg.contains("checksum")
                        || msg.contains("password")
                        || msg.contains("decrypt")
                        || msg.contains("corrupt")
                        || msg.contains("invalid")
                        || msg.contains("crc")
                        || msg.contains("data")
                    {
                        Ok(false)
                    } else {
                        Err(e).with_context(|| format!("验证 ZIP 密码时出错: {}", path.display()))
                    }
                }
            },
            Err(zip::result::ZipError::InvalidPassword) => Ok(false),
            Err(zip::result::ZipError::UnsupportedArchive(_)) => Ok(false),
            Err(e) => {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("password") || msg.contains("decrypt") || msg.contains("crc") {
                    Ok(false)
                } else {
                    Err(e).with_context(|| format!("验证 ZIP 密码时出错: {}", path.display()))
                }
            }
        }
    }
}

/// Fully read the entry so Zip's CRC32 (or AES auth) is checked.
fn read_and_verify<R: Read>(entry: &mut zip::read::ZipFile<'_, R>) -> std::io::Result<bool> {
    let expected = entry.size();
    let mut total = 0u64;
    let mut buf = [0u8; 8192];
    loop {
        match entry.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => total += n as u64,
            Err(e) => return Err(e),
        }
    }
    if expected > 0 && total != expected {
        return Ok(false);
    }
    Ok(true)
}
