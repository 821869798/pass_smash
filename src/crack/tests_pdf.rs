use std::path::PathBuf;
use std::time::Instant;

use crate::crack::charset::CharsetOptions;
use crate::crack::engine::{CrackEngine, EngineControl};
use crate::crack::handlers::pdf::PdfHandler;
use crate::crack::handlers::PasswordHandler;
use crate::crack::types::{FileKind, JobStatus};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

#[test]
fn pdf_rc4_try_password() {
    let path = fixture("sample_pdf_rc4.pdf");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = PdfHandler;
    assert!(h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "123456").unwrap());
    assert!(!h.try_password(&path, "000000").unwrap());
}

#[test]
fn pdf_aes128_try_password() {
    let path = fixture("sample_pdf_aes128.pdf");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = PdfHandler;
    assert!(h.try_password(&path, "654321").unwrap());
    assert!(!h.try_password(&path, "123456").unwrap());
}

#[test]
fn pdf_rc4_bruteforce_engine_fast() {
    let path = fixture("sample_pdf_rc4.pdf");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let opts = CharsetOptions {
        min_len: 6,
        max_len: 6,
        digits: false,
        lowercase: false,
        uppercase: false,
        symbols: false,
        custom: "123456".into(),
    };
    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());
    let result = engine.crack_file(7, path, FileKind::Pdf, &opts, |_| {});
    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("123456"));
}

/// Optional micro-benchmark (not a correctness test).
/// Run with: `cargo test --bin pass_smash pdf_rate_bench -- --ignored --nocapture`
#[test]
#[ignore = "performance micro-benchmark; too flaky for CI"]
fn pdf_rate_bench_auth_only() {
    let path = fixture("sample_pdf_rc4.pdf");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = PdfHandler;
    let _ = h.try_password(&path, "000000");
    let passwords: Vec<String> = (0..20_000u32).map(|i| format!("{i:06}")).collect();
    let t0 = Instant::now();
    for pw in &passwords {
        let _ = h.try_password(&path, pw);
    }
    let elapsed = t0.elapsed().as_secs_f64().max(1e-6);
    let rate = passwords.len() as f64 / elapsed;
    println!("auth-only rate: {rate:.0} pwd/s over {} tries", passwords.len());
    assert!(rate > 50.0, "rate unexpectedly low: {rate}");
}
