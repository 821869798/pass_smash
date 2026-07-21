use std::path::Path;

use anyhow::Context;
use sevenz_rust2::{ArchiveReader, Error as SevenZError, Password};

use super::PasswordHandler;

pub struct SevenZHandler;

impl PasswordHandler for SevenZHandler {
    fn is_encrypted(&self, path: &Path) -> anyhow::Result<bool> {
        // Opening without a password:
        // - unencrypted: succeeds
        // - header-encrypted: fails with password-related error → encrypted
        // - data-encrypted only: may still open header → try reading a stream
        match ArchiveReader::open(path, Password::empty()) {
            Ok(mut reader) => Ok(needs_password_to_read(&mut reader)),
            Err(e) if is_password_error(&e) => Ok(true),
            Err(e) => Err(e).with_context(|| format!("解析 7Z 失败: {}", path.display())),
        }
    }

    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool> {
        let pwd = Password::from(password);
        let mut reader = match ArchiveReader::open(path, pwd) {
            Ok(r) => r,
            Err(e) if is_password_error(&e) => return Ok(false),
            Err(e) => {
                return Err(e).with_context(|| format!("打开 7Z 失败: {}", path.display()));
            }
        };

        // Force content validation: open first non-directory stream entry.
        let target = reader
            .archive()
            .files
            .iter()
            .find(|f| !f.is_directory() && f.has_stream())
            .map(|f| f.name().to_string());

        let Some(name) = target else {
            // Only folders / empty — opening with password already succeeded.
            return Ok(true);
        };

        match reader.read_file(&name) {
            Ok(_) => Ok(true),
            Err(e) if is_password_error(&e) => Ok(false),
            Err(e) => {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("password")
                    || msg.contains("decrypt")
                    || msg.contains("crc")
                    || msg.contains("checksum")
                    || msg.contains("corrupt")
                    || msg.contains("invalid")
                {
                    Ok(false)
                } else {
                    Err(e).with_context(|| format!("验证 7Z 密码时出错: {}", path.display()))
                }
            }
        }
    }
}

fn needs_password_to_read(reader: &mut ArchiveReader<std::fs::File>) -> bool {
    let target = reader
        .archive()
        .files
        .iter()
        .find(|f| !f.is_directory() && f.has_stream())
        .map(|f| f.name().to_string());
    let Some(name) = target else {
        return false;
    };
    match reader.read_file(&name) {
        Ok(_) => false,
        Err(e) => is_password_error(&e),
    }
}

fn is_password_error(err: &SevenZError) -> bool {
    match err {
        SevenZError::PasswordRequired | SevenZError::MaybeBadPassword(_) => true,
        other => {
            let msg = other.to_string().to_ascii_lowercase();
            msg.contains("password") || msg.contains("decrypt") || msg.contains("encrypted")
        }
    }
}
