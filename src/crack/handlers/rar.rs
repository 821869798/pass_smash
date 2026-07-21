use std::path::Path;

use anyhow::{Context, bail};
use unrar::Archive;
use unrar::error::{Code, UnrarError, When};

use super::PasswordHandler;

pub struct RarHandler;

impl PasswordHandler for RarHandler {
    fn is_encrypted(&self, path: &Path) -> anyhow::Result<bool> {
        match Archive::new(path).open_for_processing() {
            Ok(mut archive) => {
                while let Some(header) = archive
                    .read_header()
                    .with_context(|| format!("读取 RAR 头失败: {}", path.display()))?
                {
                    if header.entry().is_file() {
                        return match header.test() {
                            Ok(_) => Ok(false),
                            Err(e) if is_password_error(&e) => Ok(true),
                            Err(e) => Err(e)
                                .with_context(|| format!("检测 RAR 加密失败: {}", path.display())),
                        };
                    }
                    archive = header
                        .skip()
                        .with_context(|| format!("跳过 RAR 条目失败: {}", path.display()))?;
                }
                Ok(false)
            }
            Err(e) if is_password_error(&e) => Ok(true),
            Err(e) => {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("password") || msg.contains("encrypt") {
                    Ok(true)
                } else {
                    Err(e).with_context(|| format!("打开 RAR 失败: {}", path.display()))
                }
            }
        }
    }

    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool> {
        let mut archive = match Archive::with_password(path, password).open_for_processing() {
            Ok(a) => a,
            Err(e) if is_password_error(&e) => return Ok(false),
            Err(e) => {
                return Err(e).with_context(|| format!("打开 RAR 失败: {}", path.display()));
            }
        };

        let mut saw_file = false;
        while let Some(header) = match archive.read_header() {
            Ok(h) => h,
            Err(e) if is_password_error(&e) => return Ok(false),
            Err(e) => {
                return Err(e).with_context(|| format!("读取 RAR 头失败: {}", path.display()));
            }
        } {
            if header.entry().is_file() {
                saw_file = true;
                return match header.test() {
                    Ok(_) => Ok(true),
                    Err(e) if is_password_error(&e) => Ok(false),
                    Err(e) => {
                        let msg = e.to_string().to_ascii_lowercase();
                        if msg.contains("password")
                            || msg.contains("crc")
                            || msg.contains("checksum")
                            || msg.contains("bad data")
                            || msg.contains("corrupt")
                        {
                            Ok(false)
                        } else {
                            Err(e).with_context(|| {
                                format!("验证 RAR 密码时出错: {}", path.display())
                            })
                        }
                    }
                };
            }
            archive = match header.skip() {
                Ok(a) => a,
                Err(e) if is_password_error(&e) => return Ok(false),
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("跳过 RAR 条目失败: {}", path.display()));
                }
            };
        }

        if !saw_file {
            return Ok(true);
        }
        bail!("RAR 中未找到可验证的文件条目");
    }
}

fn is_password_error(err: &UnrarError) -> bool {
    match err.code {
        Code::BadPassword | Code::MissingPassword => true,
        Code::BadData if matches!(err.when, When::Process | When::Open) => true,
        _ => {
            let msg = err.to_string().to_ascii_lowercase();
            msg.contains("password") || msg.contains("encrypt")
        }
    }
}
