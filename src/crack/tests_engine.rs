//! End-to-end smoke tests for the crack engine (no UI).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::crack::charset::CharsetOptions;
use crate::crack::engine::{CrackEngine, EngineControl};
use crate::crack::types::{FileKind, JobStatus};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn crack_zipcrypto_digits() {
    let path = fixture("sample_zipcrypto.zip");
    assert!(path.is_file(), "missing fixture: {path:?}");

    let opts = CharsetOptions {
        min_len: 1,
        max_len: 2,
        digits: true,
        lowercase: false,
        uppercase: false,
        symbols: false,
        custom: String::new(),
    };

    let engine = CrackEngine::new(EngineControl::new(), 2);
    let progress_count = Arc::new(AtomicU32::new(0));
    let progress_count_cb = Arc::clone(&progress_count);
    let result = engine.crack_file(1, path, FileKind::Zip, &opts, move |_| {
        progress_count_cb.fetch_add(1, Ordering::Relaxed);
    });

    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("42"));
    assert!(
        progress_count.load(Ordering::Relaxed) >= 1,
        "expected live progress callbacks"
    );
}

#[test]
fn crack_aes_zip_digits() {
    let path = fixture("sample_aes.zip");
    assert!(path.is_file(), "missing fixture: {path:?}");

    let opts = CharsetOptions {
        min_len: 2,
        max_len: 2,
        digits: true,
        ..Default::default()
    };

    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());
    let result = engine.crack_file(2, path, FileKind::Zip, &opts, |_| {});
    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("42"));
}

#[test]
fn progress_callbacks_fire_during_long_run() {
    let path = fixture("sample_aes.zip");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let opts = CharsetOptions {
        min_len: 1,
        max_len: 3,
        digits: true,
        ..Default::default()
    };
    let max_tried = Arc::new(AtomicU32::new(0));
    let max_tried_cb = Arc::clone(&max_tried);
    let engine = CrackEngine::new(EngineControl::new(), 2);
    let _ = engine.crack_file(3, path, FileKind::Zip, &opts, move |p| {
        let prev = max_tried_cb.load(Ordering::Relaxed);
        if p.tried as u32 > prev {
            max_tried_cb.store(p.tried as u32, Ordering::Relaxed);
        }
    });
    assert!(max_tried.load(Ordering::Relaxed) > 0);
}
