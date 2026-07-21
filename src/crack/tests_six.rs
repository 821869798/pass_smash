use std::path::PathBuf;

use crate::crack::charset::{CharsetOptions, PasswordGenerator};
use crate::crack::engine::{CrackEngine, EngineControl};
use crate::crack::handlers::zip::ZipHandler;
use crate::crack::handlers::PasswordHandler;
use crate::crack::types::{FileKind, JobStatus};

#[test]
fn six_digit_space_is_one_million() {
    let opts = CharsetOptions {
        min_len: 6,
        max_len: 6,
        digits: true,
        lowercase: false,
        uppercase: false,
        symbols: false,
        custom: String::new(),
    };
    assert_eq!(opts.total_candidates(), 1_000_000);
    let mut generator = PasswordGenerator::new(&opts).unwrap();
    assert_eq!(generator.next().as_deref(), Some("000000"));
    assert_eq!(
        PasswordGenerator::new(&opts).unwrap().nth(123_456).unwrap(),
        "123456"
    );
}

#[test]
fn try_password_6digit_direct() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample_6digit.zip");
    let h = ZipHandler;
    assert!(h.try_password(&path, "123456").unwrap());
    assert!(!h.try_password(&path, "000000").unwrap());
    // Known ZipCrypto false-positive candidate style: must NOT accept wrong CRC
    assert!(!h.try_password(&path, "101568").unwrap());
}

#[test]
fn zipcrypto_rejects_false_positive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample_6digit_early.zip");
    let h = ZipHandler;
    assert!(h.try_password(&path, "000042").unwrap());
    // Previously accepted by weak traditional check + partial read
    assert!(!h.try_password(&path, "101568").unwrap());
}

#[test]
fn crack_6digit_early_password() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample_6digit_early.zip");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let opts = CharsetOptions {
        min_len: 6,
        max_len: 6,
        digits: true,
        lowercase: false,
        uppercase: false,
        symbols: false,
        custom: String::new(),
    };
    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());
    let result = engine.crack_file(42, path, FileKind::Zip, &opts, |_| {});
    assert_eq!(result.status, JobStatus::Found, "msg={}", result.message);
    assert_eq!(result.password.as_deref(), Some("000042"));
}
