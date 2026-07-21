//! Per-format password verification handlers.

pub mod pdf;
pub mod rar;
pub mod sevenz;
pub mod zip;
pub mod office;

use std::path::Path;

use super::types::FileKind;

/// Trait for format-specific password verification.
pub trait PasswordHandler: Send + Sync {
    fn try_password(&self, path: &Path, password: &str) -> anyhow::Result<bool>;

    fn is_encrypted(&self, _path: &Path) -> anyhow::Result<bool> {
        Ok(true)
    }
}

pub fn handler_for(kind: FileKind) -> Option<Box<dyn PasswordHandler>> {
    match kind {
        FileKind::Zip => Some(Box::new(zip::ZipHandler)),
        FileKind::Pdf => Some(Box::new(pdf::PdfHandler)),
        FileKind::SevenZ => Some(Box::new(sevenz::SevenZHandler)),
        FileKind::Rar => Some(Box::new(rar::RarHandler)),
        FileKind::Office => Some(Box::new(office::OfficeHandler)),
        FileKind::Unknown => None,
    }
}
