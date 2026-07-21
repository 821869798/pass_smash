use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::i18n::Locale;

/// Supported file kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Zip,
    Pdf,
    SevenZ,
    Rar,
    Office,
    Unknown,
}

impl FileKind {
    pub fn detect(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "zip" | "zipx" => Self::Zip,
            "pdf" => Self::Pdf,
            "7z" => Self::SevenZ,
            "rar" => Self::Rar,
            // Modern OOXML + legacy Word binary (RC4 CryptoAPI)
            "docx" | "docm" | "xlsx" | "xlsm" | "pptx" | "pptm" | "doc" => Self::Office,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Pdf => "PDF",
            Self::SevenZ => "7Z",
            Self::Rar => "RAR",
            Self::Office => "Office",
            Self::Unknown => "?",
        }
    }

    pub fn is_supported(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone)]
pub struct TargetFile {
    pub path: PathBuf,
    pub kind: FileKind,
}

impl TargetFile {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let kind = FileKind::detect(&path);
        Self { path, kind }
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Found,
    Exhausted,
    Failed,
    Cancelled,
    Skipped,
}

impl JobStatus {
    pub fn label(self, locale: Locale) -> &'static str {
        match (self, locale) {
            (Self::Pending, Locale::Zh) => "等待中",
            (Self::Pending, Locale::En) => "Pending",
            (Self::Running, Locale::Zh) => "破解中",
            (Self::Running, Locale::En) => "Running",
            (Self::Found, Locale::Zh) => "已找到",
            (Self::Found, Locale::En) => "Found",
            (Self::Exhausted, Locale::Zh) => "已穷尽",
            (Self::Exhausted, Locale::En) => "Exhausted",
            (Self::Failed, Locale::Zh) => "失败",
            (Self::Failed, Locale::En) => "Failed",
            (Self::Cancelled, Locale::Zh) => "已取消",
            (Self::Cancelled, Locale::En) => "Cancelled",
            (Self::Skipped, Locale::Zh) => "已跳过",
            (Self::Skipped, Locale::En) => "Skipped",
        }
    }

}

#[derive(Debug, Clone)]
pub struct CrackJob {
    pub id: u64,
    pub file: TargetFile,
    pub status: JobStatus,
    pub password: Option<String>,
    pub tried: u64,
    pub total: u64,
    pub rate: f64,
    pub elapsed: Duration,
    pub message: String,
}

impl CrackJob {
    pub fn new(id: u64, file: TargetFile) -> Self {
        Self {
            id,
            file,
            status: JobStatus::Pending,
            password: None,
            tried: 0,
            total: 0,
            rate: 0.0,
            elapsed: Duration::ZERO,
            message: String::new(),
        }
    }

    pub fn progress_ratio(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.tried as f64 / self.total as f64).clamp(0.0, 1.0) as f32
    }
}

#[derive(Debug, Clone)]
pub struct CrackResult {
    pub job_id: u64,
    pub status: JobStatus,
    pub password: Option<String>,
    pub tried: u64,
    pub total: u64,
    pub elapsed: Duration,
    pub message: String,
}
