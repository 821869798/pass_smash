//! Multi-threaded brute-force engine with cancellation and progress reporting.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rayon::prelude::*;

use super::charset::{CharsetOptions, PasswordGenerator};
use super::handlers::{self, PasswordHandler};
use super::types::{CrackResult, FileKind, JobStatus};

/// Shared control flags for a running batch.
#[derive(Clone, Default)]
pub struct EngineControl {
    cancel: Arc<AtomicBool>,
}

impl EngineControl {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

/// Progress snapshot emitted while cracking.
#[derive(Debug, Clone)]
pub struct CrackProgress {
    pub job_id: u64,
    pub tried: u64,
    pub total: u64,
    pub rate: f64,
    pub elapsed: Duration,
    pub current_password: String,
    pub found: Option<String>,
    pub finished: bool,
    pub status: JobStatus,
    pub message: String,
}

pub struct CrackEngine {
    control: EngineControl,
    threads: usize,
}

impl CrackEngine {
    pub fn new(control: EngineControl, threads: usize) -> Self {
        Self {
            control,
            threads: threads.max(1),
        }
    }

    /// Auto thread count: physical-ish default from CPU count (at least 1).
    pub fn auto_threads() -> usize {
        num_cpus::get().max(1)
    }

    /// Crack a single file. Invokes `on_progress` periodically (thread-safe callback).
    pub fn crack_file<F>(
        &self,
        job_id: u64,
        path: PathBuf,
        kind: FileKind,
        opts: &CharsetOptions,
        on_progress: F,
    ) -> CrackResult
    where
        F: Fn(CrackProgress) + Send + Sync + 'static,
    {
        let start = Instant::now();
        let on_progress = Arc::new(on_progress);

        let handler = match handlers::handler_for(kind) {
            Some(h) => h,
            None => {
                let result = CrackResult {
                    job_id,
                    status: JobStatus::Skipped,
                    password: None,
                    tried: 0,
                    total: 0,
                    elapsed: start.elapsed(),
                    message: format!("unsupported file type: {}", kind.label()),
                };
                on_progress(final_progress(&result, String::new()));
                return result;
            }
        };

        // Unencrypted short-circuit.
        match handler.is_encrypted(&path) {
            Ok(false) => {
                let result = CrackResult {
                    job_id,
                    status: JobStatus::Found,
                    password: Some(String::new()),
                    tried: 0,
                    total: 0,
                    elapsed: start.elapsed(),
                    message: "文件未加密".into(),
                };
                on_progress(final_progress(&result, String::new()));
                return result;
            }
            Ok(true) => {}
            Err(e) => {
                let result = CrackResult {
                    job_id,
                    status: JobStatus::Failed,
                    password: None,
                    tried: 0,
                    total: 0,
                    elapsed: start.elapsed(),
                    message: format!("检测加密状态失败: {e}"),
                };
                on_progress(final_progress(&result, String::new()));
                return result;
            }
        }

        let generator = match PasswordGenerator::new(opts) {
            Ok(g) => g,
            Err(e) => {
                let result = CrackResult {
                    job_id,
                    status: JobStatus::Failed,
                    password: None,
                    tried: 0,
                    total: 0,
                    elapsed: start.elapsed(),
                    message: e,
                };
                on_progress(final_progress(&result, String::new()));
                return result;
            }
        };

        let total = generator.total();
        let tried = Arc::new(AtomicU64::new(0));
        let found = Arc::new(Mutex::new(None::<String>));
        let last_password = Arc::new(Mutex::new(String::new()));
        let fatal = Arc::new(Mutex::new(None::<String>));

        // Emit "starting" progress immediately so the UI bar leaves 0%.
        on_progress(CrackProgress {
            job_id,
            tried: 0,
            total,
            rate: 0.0,
            elapsed: Duration::ZERO,
            current_password: String::new(),
            found: None,
            finished: false,
            status: JobStatus::Running,
            message: "开始枚举…".into(),
        });

        if total > 50_000_000 {
            let result = CrackResult {
                job_id,
                status: JobStatus::Failed,
                password: None,
                tried: 0,
                total,
                elapsed: start.elapsed(),
                message: format!(
                    "搜索空间过大 ({total} 个候选)。请缩小长度范围或字符集后重试。"
                ),
            };
            on_progress(final_progress(&result, String::new()));
            return result;
        }

        let passwords: Vec<String> = generator.collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.threads)
            .build()
            .expect("rayon pool");

        let control = self.control.clone();
        let path_arc = Arc::new(path);
        let handler: Arc<dyn PasswordHandler> = Arc::from(handler);

        // Progress reporter: push live updates into the UI callback.
        let tried_r = Arc::clone(&tried);
        let found_r = Arc::clone(&found);
        let last_r = Arc::clone(&last_password);
        let control_r = control.clone();
        let stop_reporter = Arc::new(AtomicBool::new(false));
        let stop_reporter_r = Arc::clone(&stop_reporter);
        let on_progress_r = Arc::clone(&on_progress);

        let reporter = std::thread::spawn(move || {
            while !stop_reporter_r.load(Ordering::SeqCst) {
                let t = tried_r.load(Ordering::Relaxed);
                let elapsed = start.elapsed();
                let rate = if elapsed.as_secs_f64() > 0.0 {
                    t as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                let current = last_r.lock().clone();
                let found_pw = found_r.lock().clone();
                let status = if found_pw.is_some() {
                    JobStatus::Found
                } else if control_r.is_cancelled() {
                    JobStatus::Cancelled
                } else {
                    JobStatus::Running
                };
                on_progress_r(CrackProgress {
                    job_id,
                    tried: t,
                    total,
                    rate,
                    elapsed,
                    current_password: current,
                    found: found_pw,
                    finished: false,
                    status,
                    message: String::new(),
                });
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        pool.install(|| {
            let _ = passwords.par_iter().try_for_each(|password| {
                if control.is_cancelled() || found.lock().is_some() {
                    return Err(());
                }

                *last_password.lock() = password.clone();

                match handler.try_password(&path_arc, password) {
                    Ok(true) => {
                        *found.lock() = Some(password.clone());
                        tried.fetch_add(1, Ordering::Relaxed);
                        Err(())
                    }
                    Ok(false) => {
                        tried.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let lower = msg.to_ascii_lowercase();
                        if lower.contains("password")
                            || lower.contains("decrypt")
                            || lower.contains("crc")
                        {
                            tried.fetch_add(1, Ordering::Relaxed);
                            Ok(())
                        } else {
                            *fatal.lock() = Some(msg);
                            Err(())
                        }
                    }
                }
            });
        });

        stop_reporter.store(true, Ordering::SeqCst);
        let _ = reporter.join();

        let elapsed = start.elapsed();
        let tried_n = tried.load(Ordering::Relaxed);
        let found_pw = found.lock().clone();

        if let Some(msg) = fatal.lock().clone() {
            let result = CrackResult {
                job_id,
                status: JobStatus::Failed,
                password: None,
                tried: tried_n,
                total,
                elapsed,
                message: msg,
            };
            on_progress(final_progress(&result, String::new()));
            return result;
        }

        if let Some(pw) = found_pw {
            let result = CrackResult {
                job_id,
                status: JobStatus::Found,
                password: Some(pw.clone()),
                tried: tried_n,
                total,
                elapsed,
                message: format!("密码: {pw}"),
            };
            on_progress(final_progress(&result, pw));
            return result;
        }

        if control.is_cancelled() {
            let result = CrackResult {
                job_id,
                status: JobStatus::Cancelled,
                password: None,
                tried: tried_n,
                total,
                elapsed,
                message: "用户取消".into(),
            };
            on_progress(final_progress(&result, String::new()));
            return result;
        }

        let result = CrackResult {
            job_id,
            status: JobStatus::Exhausted,
            password: None,
            tried: tried_n,
            total,
            elapsed,
            message: "已穷尽所有候选密码".into(),
        };
        on_progress(final_progress(&result, String::new()));
        result
    }
}

fn final_progress(result: &CrackResult, current: String) -> CrackProgress {
    let rate = if result.elapsed.as_secs_f64() > 0.0 {
        result.tried as f64 / result.elapsed.as_secs_f64()
    } else {
        0.0
    };
    CrackProgress {
        job_id: result.job_id,
        tried: result.tried,
        total: result.total,
        rate,
        elapsed: result.elapsed,
        current_password: current,
        found: result.password.clone(),
        finished: true,
        status: result.status,
        message: result.message.clone(),
    }
}
