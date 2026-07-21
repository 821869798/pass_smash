use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use anyhow::Context;
use lopdf::Document;
use parking_lot::RwLock;

use super::PasswordHandler;

/// Cache parsed encrypted PDFs (structure only) so each password try is auth-only.
/// Document is not mutated during authenticate_*, so shared reads are fine.
static PDF_CACHE: LazyLock<RwLock<HashMap<String, Arc<CachedPdf>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

struct CachedPdf {
    /// `None` means the PDF is not encrypted.
    doc: Option<Document>,
}

pub struct PdfHandler;

impl PdfHandler {
    fn cached(&self, path: &Path) -> anyhow::Result<Arc<CachedPdf>> {
        let key = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned();

        if let Some(v) = PDF_CACHE.read().get(&key).cloned() {
            return Ok(v);
        }

        let doc = Document::load(path)
            .with_context(|| format!("解析 PDF 失败: {}", path.display()))?;

        let cached = if doc.is_encrypted() {
            Arc::new(CachedPdf { doc: Some(doc) })
        } else {
            Arc::new(CachedPdf { doc: None })
        };
        PDF_CACHE.write().insert(key, Arc::clone(&cached));
        Ok(cached)
    }
}

impl PasswordHandler for PdfHandler {
    fn is_encrypted(&self, path: &Path) -> anyhow::Result<bool> {
        Ok(self.cached(path)?.doc.is_some())
    }

    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool> {
        let cached = self.cached(path)?;
        let Some(doc) = cached.doc.as_ref() else {
            return Ok(true); // not encrypted
        };

        // lopdf 0.44+: authenticate without decrypting object streams.
        match doc.authenticate_password(password) {
            Ok(()) => Ok(true),
            Err(e) if is_wrong_password(&e) => Ok(false),
            Err(e) if is_unsupported(&e) => Err(e).with_context(|| {
                format!(
                    "PDF 加密方式暂不支持: {}",
                    path.display()
                )
            }),
            Err(e) => {
                // Treat remaining auth/crypto failures as wrong password so
                // brute-force can continue; hard IO/parse errors bubble up.
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("encrypt") || msg.contains("crypt") || msg.contains("auth") {
                    Ok(false)
                } else {
                    Err(e).with_context(|| format!("验证 PDF 密码时出错: {}", path.display()))
                }
            }
        }
    }
}

fn is_wrong_password(err: &lopdf::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("password")
        || msg.contains("incorrect")
        || msg.contains("authentication")
        || msg.contains("not authenticated")
        || msg.contains("wrong")
}

fn is_unsupported(err: &lopdf::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("unsupported")
        || msg.contains("not implemented")
        || msg.contains("not support")
}

/// Clear cache (e.g. between tests).
#[cfg(test)]
#[allow(dead_code)]
pub fn clear_cache() {
    PDF_CACHE.write().clear();
}
