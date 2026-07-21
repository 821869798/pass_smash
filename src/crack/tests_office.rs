use std::path::PathBuf;

use crate::crack::handlers::office::OfficeHandler;
use crate::crack::handlers::PasswordHandler;
use crate::crack::types::FileKind;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

#[test]
fn office_detect_extensions() {
    assert_eq!(FileKind::detect(PathBuf::from("a.docx").as_path()), FileKind::Office);
    assert_eq!(FileKind::detect(PathBuf::from("a.xlsx").as_path()), FileKind::Office);
    assert_eq!(FileKind::detect(PathBuf::from("a.pptx").as_path()), FileKind::Office);
    assert_eq!(FileKind::detect(PathBuf::from("a.doc").as_path()), FileKind::Office);
}

#[test]
fn office_plain_not_encrypted() {
    let path = fixture("sample_office_plain.docx");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = OfficeHandler;
    assert!(!h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "anything").unwrap());
}

#[test]
fn office_standard_password() {
    let path = fixture("sample_office_enc.docx");
    assert!(path.is_file(), "missing fixture: {path:?}");
    let h = OfficeHandler;
    assert!(h.is_encrypted(&path).unwrap());
    assert!(h.try_password(&path, "Password1234_").unwrap());
    assert!(!h.try_password(&path, "wrong").unwrap());
    assert!(!h.try_password(&path, "Password1234").unwrap());
}

#[test]
fn office_agile_password() {
    let path = fixture("sample_office_agile.docx");
    assert!(path.is_file() && path.metadata().map(|m| m.len() > 100).unwrap_or(false), "missing/invalid fixture: {path:?}");
    let h = OfficeHandler;
    assert!(h.is_encrypted(&path).unwrap());
    // same password used in office-crypto docs/examples for standard; agile fixture often same
    assert!(h.try_password(&path, "testPassword").unwrap());
    assert!(!h.try_password(&path, "Password1234_").unwrap());
}
