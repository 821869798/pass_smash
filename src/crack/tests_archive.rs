use std::path::PathBuf;

use crate::crack::charset::CharsetOptions;
use crate::crack::engine::{CrackEngine, EngineControl};
use crate::crack::handlers::rar::RarHandler;
use crate::crack::handlers::sevenz::SevenZHandler;
use crate::crack::handlers::PasswordHandler;
use crate::crack::types::{FileKind, JobStatus};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

#[test]
fn sevenz_try_password() {
    let path = fixture("sample_7z.7z");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = SevenZHandler;
    assert!(h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "42").unwrap());
    assert!(!h.try_password(&path, "00").unwrap());
}

#[test]
fn sevenz_aes_try_password() {
    // Password from sevenz-rust2 test suite
    let path = fixture("sample_7z_aes.7z");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = SevenZHandler;
    assert!(h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "iBlm8NTigvru0Jr0").unwrap());
    assert!(!h.try_password(&path, "wrong").unwrap());
}

#[test]
fn sevenz_engine_digits() {
    let path = fixture("sample_7z.7z");
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
    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());
    let result = engine.crack_file(1, path, FileKind::SevenZ, &opts, |_| {});
    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("42"));
}

#[test]
fn rar_try_password() {
    // unrar test fixture password is "unrar"
    let path = fixture("sample_rar_crypted.rar");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = RarHandler;
    assert!(h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "unrar").unwrap());
    assert!(!h.try_password(&path, "wrong").unwrap());
}

#[test]
fn rar_engine_custom_charset() {
    let path = fixture("sample_rar_crypted.rar");
    assert!(path.is_file(), "missing fixture: {path:?}");
    // charset of letters in "unrar" only: u,n,r,a
    let opts = CharsetOptions {
        min_len: 5,
        max_len: 5,
        digits: false,
        lowercase: false,
        uppercase: false,
        symbols: false,
        custom: "unra".into(),
    };
    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());
    let result = engine.crack_file(2, path, FileKind::Rar, &opts, |_| {});
    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("unrar"));
}
