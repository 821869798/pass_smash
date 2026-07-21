//! Simple process-wide UI locale (Chinese / English).

use std::sync::atomic::{AtomicU8, Ordering};

static LOCALE: AtomicU8 = AtomicU8::new(0); // 0 = Zh, 1 = En

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Locale {
    Zh = 0,
    En = 1,
}

impl Locale {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Zh => Self::En,
            Self::En => Self::Zh,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "English",
        }
    }
}

pub fn current() -> Locale {
    match LOCALE.load(Ordering::Relaxed) {
        1 => Locale::En,
        _ => Locale::Zh,
    }
}

pub fn set(locale: Locale) {
    LOCALE.store(locale as u8, Ordering::Relaxed);
}

/// UI string table keyed by logical id.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    AppSubtitle,
    AddFiles,
    Clear,
    CrackParams,
    Digits,
    Lowercase,
    Uppercase,
    Symbols,
    MinLen,
    MaxLen,
    CustomChars,
    Start,
    Cracking,
    Stop,
    Ready,
    PleaseAddFiles,
    AddedFiles,
    Cleared,
    CannotClearWhileRunning,
    CannotAddWhileRunning,
    Starting,
    RunningStatus,
    JobFinished,
    BatchDone,
    FileList,
    SupportedFormats,
    EmptyTitle,
    EmptyHint,
    Remove,
    Password,
    NoPassword,
    Queued,
    Trying,
    UnsupportedType,
    SearchHint,
    LangSwitch,
}

impl Msg {
    pub fn get(self, locale: Locale) -> &'static str {
        match (self, locale) {
            (Self::AppSubtitle, Locale::Zh) => "压缩包 / PDF / Office 密码解锁工具",
            (Self::AppSubtitle, Locale::En) => "Archive / PDF / Office password unlocker",
            (Self::AddFiles, Locale::Zh) => "添加文件",
            (Self::AddFiles, Locale::En) => "Add files",
            (Self::Clear, Locale::Zh) => "清空",
            (Self::Clear, Locale::En) => "Clear",
            (Self::CrackParams, Locale::Zh) => "破解参数",
            (Self::CrackParams, Locale::En) => "Crack options",
            (Self::Digits, Locale::Zh) => "数字 0-9",
            (Self::Digits, Locale::En) => "Digits 0-9",
            (Self::Lowercase, Locale::Zh) => "小写 a-z",
            (Self::Lowercase, Locale::En) => "Lowercase a-z",
            (Self::Uppercase, Locale::Zh) => "大写 A-Z",
            (Self::Uppercase, Locale::En) => "Uppercase A-Z",
            (Self::Symbols, Locale::Zh) => "符号",
            (Self::Symbols, Locale::En) => "Symbols",
            (Self::MinLen, Locale::Zh) => "最短长度",
            (Self::MinLen, Locale::En) => "Min length",
            (Self::MaxLen, Locale::Zh) => "最长长度",
            (Self::MaxLen, Locale::En) => "Max length",
            (Self::CustomChars, Locale::Zh) => "自定义字符（可选）",
            (Self::CustomChars, Locale::En) => "Custom chars (optional)",
            (Self::Start, Locale::Zh) => "开始破解",
            (Self::Start, Locale::En) => "Start",
            (Self::Cracking, Locale::Zh) => "破解中…",
            (Self::Cracking, Locale::En) => "Cracking…",
            (Self::Stop, Locale::Zh) => "停止",
            (Self::Stop, Locale::En) => "Stop",
            (Self::Ready, Locale::Zh) => "就绪 — 添加文件后点击「开始破解」",
            (Self::Ready, Locale::En) => "Ready — add files, then click Start",
            (Self::PleaseAddFiles, Locale::Zh) => "请先添加文件",
            (Self::PleaseAddFiles, Locale::En) => "Please add files first",
            (Self::AddedFiles, Locale::Zh) => "已添加",
            (Self::AddedFiles, Locale::En) => "Added",
            (Self::Cleared, Locale::Zh) => "已清空文件列表",
            (Self::Cleared, Locale::En) => "File list cleared",
            (Self::CannotClearWhileRunning, Locale::Zh) => "破解进行中，无法清空（请先停止）",
            (Self::CannotClearWhileRunning, Locale::En) => {
                "Cannot clear while cracking (stop first)"
            }
            (Self::CannotAddWhileRunning, Locale::Zh) => "破解进行中，无法添加文件（请先停止）",
            (Self::CannotAddWhileRunning, Locale::En) => {
                "Cannot add files while cracking (stop first)"
            }
            (Self::Starting, Locale::Zh) => "开始破解…",
            (Self::Starting, Locale::En) => "Starting…",
            (Self::RunningStatus, Locale::Zh) => "破解中…",
            (Self::RunningStatus, Locale::En) => "Cracking…",
            (Self::JobFinished, Locale::Zh) => "任务",
            (Self::JobFinished, Locale::En) => "Job",
            (Self::BatchDone, Locale::Zh) => "全部完成 — 成功",
            (Self::BatchDone, Locale::En) => "All done — success",
            (Self::FileList, Locale::Zh) => "文件列表",
            (Self::FileList, Locale::En) => "Files",
            (Self::SupportedFormats, Locale::Zh) => "支持: ZIP · 7Z · RAR · PDF · Office",
            (Self::SupportedFormats, Locale::En) => "Supported: ZIP · 7Z · RAR · PDF · Office",
            (Self::EmptyTitle, Locale::Zh) => "尚未添加文件",
            (Self::EmptyTitle, Locale::En) => "No files yet",
            (Self::EmptyHint, Locale::Zh) => {
                "点击「添加文件」，或直接把 ZIP / 7Z / RAR / PDF / Office 拖进窗口"
            }
            (Self::EmptyHint, Locale::En) => {
                "Click “Add files”, or drag ZIP / 7Z / RAR / PDF / Office into the window"
            }
            (Self::Remove, Locale::Zh) => "移除",
            (Self::Remove, Locale::En) => "Remove",
            (Self::Password, Locale::Zh) => "密码",
            (Self::Password, Locale::En) => "Password",
            (Self::NoPassword, Locale::Zh) => "(无密码)",
            (Self::NoPassword, Locale::En) => "(no password)",
            (Self::Queued, Locale::Zh) => "排队中…",
            (Self::Queued, Locale::En) => "Queued…",
            (Self::Trying, Locale::Zh) => "尝试",
            (Self::Trying, Locale::En) => "Trying",
            (Self::UnsupportedType, Locale::Zh) => "不支持的文件类型",
            (Self::UnsupportedType, Locale::En) => "Unsupported file type",
            (Self::SearchHint, Locale::Zh) => "字符集",
            (Self::SearchHint, Locale::En) => "Charset",
            (Self::LangSwitch, Locale::Zh) => "中 / EN",
            (Self::LangSwitch, Locale::En) => "EN / 中",
        }
    }

    pub fn t(self) -> &'static str {
        self.get(current())
    }
}

pub fn t(msg: Msg) -> &'static str {
    msg.t()
}

pub fn format_search_hint(
    locale: Locale,
    charset_len: usize,
    total_fmt: &str,
    min_len: usize,
    max_len: usize,
    threads: usize,
) -> String {
    match locale {
        Locale::Zh => format!(
            "字符集 {charset_len} 个 · 候选 {total_fmt} 次 · 长度 {min_len}–{max_len} · 自动 {threads} 线程"
        ),
        Locale::En => format!(
            "Charset {charset_len} · candidates {total_fmt} · length {min_len}–{max_len} · auto {threads} threads"
        ),
    }
}

pub fn format_added(locale: Locale, added: usize, total: usize) -> String {
    match locale {
        Locale::Zh => format!("已添加 {added} 个文件，当前共 {total} 个"),
        Locale::En => format!("Added {added} file(s), {total} total"),
    }
}

pub fn format_starting(locale: Locale, threads: usize) -> String {
    match locale {
        Locale::Zh => format!("开始破解…（{threads} 线程）"),
        Locale::En => format!("Starting… ({threads} threads)"),
    }
}

pub fn format_running(locale: Locale, rate: f64, tried: u64, total: u64) -> String {
    match locale {
        Locale::Zh => format!("破解中… {rate:.0} pwd/s  已试 {tried}/{total}"),
        Locale::En => format!("Cracking… {rate:.0} pwd/s  tried {tried}/{total}"),
    }
}

pub fn format_job_finished(
    locale: Locale,
    id: u64,
    status: &str,
    message: &str,
) -> String {
    match locale {
        Locale::Zh => format!("任务 #{id} {status} — {message}"),
        Locale::En => format!("Job #{id} {status} — {message}"),
    }
}

pub fn format_batch_done(locale: Locale, found: usize, total: usize) -> String {
    match locale {
        Locale::Zh => format!("全部完成 — 成功 {found}/{total}"),
        Locale::En => format!("All done — success {found}/{total}"),
    }
}

pub fn format_password(locale: Locale, password: &str) -> String {
    if password.is_empty() {
        return Msg::NoPassword.get(locale).to_string();
    }
    match locale {
        Locale::Zh => format!("密码: {password}"),
        Locale::En => format!("Password: {password}"),
    }
}

pub fn format_trying(locale: Locale, password: &str) -> String {
    match locale {
        Locale::Zh => format!("尝试: {password}"),
        Locale::En => format!("Trying: {password}"),
    }
}

pub fn format_file_list_title(locale: Locale, count: usize) -> String {
    match locale {
        Locale::Zh => format!("文件列表 ({count})"),
        Locale::En => format!("Files ({count})"),
    }
}

pub fn open_dialog_title(locale: Locale) -> &'static str {
    match locale {
        Locale::Zh => "选择要解锁的文件",
        Locale::En => "Select files to unlock",
    }
}

pub fn filter_supported(locale: Locale) -> &'static str {
    match locale {
        Locale::Zh => "支持的文件",
        Locale::En => "Supported files",
    }
}

pub fn filter_all(locale: Locale) -> &'static str {
    match locale {
        Locale::Zh => "所有文件",
        Locale::En => "All files",
    }
}
