//! Cross-format smoke matrix: every supported kind with a real fixture.

use std::path::PathBuf;

use crate::crack::charset::CharsetOptions;
use crate::crack::engine::{CrackEngine, EngineControl};
use crate::crack::handlers::office::OfficeHandler;
use crate::crack::handlers::pdf::PdfHandler;
use crate::crack::handlers::rar::RarHandler;
use crate::crack::handlers::sevenz::SevenZHandler;
use crate::crack::handlers::zip::ZipHandler;
use crate::crack::handlers::PasswordHandler;
use crate::crack::types::{FileKind, JobStatus};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn require(name: &str) -> PathBuf {
    let path = fixture(name);
    assert!(
        path.is_file(),
        "missing fixture {name} at {path:?}. See fixtures/README.md"
    );
    path
}

struct Case {
    name: &'static str,
    kind: FileKind,
    password: &'static str,
    wrong: &'static str,
}

#[test]
fn all_formats_try_password_matrix() {
    let cases = [
        Case {
            name: "sample_zipcrypto.zip",
            kind: FileKind::Zip,
            password: "42",
            wrong: "00",
        },
        Case {
            name: "sample_aes.zip",
            kind: FileKind::Zip,
            password: "42",
            wrong: "99",
        },
        Case {
            name: "sample_7z.7z",
            kind: FileKind::SevenZ,
            password: "42",
            wrong: "00",
        },
        Case {
            name: "sample_7z_aes.7z",
            kind: FileKind::SevenZ,
            password: "iBlm8NTigvru0Jr0",
            wrong: "wrong",
        },
        Case {
            name: "sample_rar_crypted.rar",
            kind: FileKind::Rar,
            password: "unrar",
            wrong: "wrong",
        },
        Case {
            name: "sample_pdf_rc4.pdf",
            kind: FileKind::Pdf,
            password: "123456",
            wrong: "000000",
        },
        Case {
            name: "sample_pdf_aes128.pdf",
            kind: FileKind::Pdf,
            password: "654321",
            wrong: "000000",
        },
        Case {
            name: "sample_office_enc.docx",
            kind: FileKind::Office,
            password: "Password1234_",
            wrong: "wrong",
        },
        Case {
            name: "sample_office_agile.docx",
            kind: FileKind::Office,
            password: "testPassword",
            wrong: "Password1234_",
        },
    ];

    for case in cases {
        let path = require(case.name);
        assert_eq!(
            FileKind::detect(&path),
            case.kind,
            "detect mismatch for {}",
            case.name
        );

        let handler: Box<dyn PasswordHandler> = match case.kind {
            FileKind::Zip => Box::new(ZipHandler),
            FileKind::SevenZ => Box::new(SevenZHandler),
            FileKind::Rar => Box::new(RarHandler),
            FileKind::Pdf => Box::new(PdfHandler),
            FileKind::Office => Box::new(OfficeHandler),
            FileKind::Unknown => panic!("unknown"),
        };

        assert!(
            handler.is_encrypted(&path).unwrap(),
            "{} should be encrypted",
            case.name
        );
        assert!(
            handler.try_password(&path, case.password).unwrap(),
            "{} correct password rejected: {}",
            case.name,
            case.password
        );
        assert!(
            !handler.try_password(&path, case.wrong).unwrap(),
            "{} wrong password accepted: {}",
            case.name,
            case.wrong
        );
        eprintln!("OK {} ({:?}) password={}", case.name, case.kind, case.password);
    }
}

#[test]
fn all_formats_engine_finds_password() {
    // Short-space engine cracks for each format with small custom charset / digits.
    let engine = CrackEngine::new(EngineControl::new(), CrackEngine::auto_threads());

    // ZIP password 42
    {
        let path = require("sample_zipcrypto.zip");
        let opts = CharsetOptions {
            min_len: 1,
            max_len: 2,
            digits: true,
            lowercase: false,
            uppercase: false,
            symbols: false,
            custom: String::new(),
        };
        let r = engine.crack_file(10, path, FileKind::Zip, &opts, |_| {});
        assert_eq!(r.status, JobStatus::Found, "zip: {}", r.message);
        assert_eq!(r.password.as_deref(), Some("42"));
    }

    // 7z password 42
    {
        let path = require("sample_7z.7z");
        let opts = CharsetOptions {
            min_len: 1,
            max_len: 2,
            digits: true,
            ..Default::default()
        };
        let r = engine.crack_file(11, path, FileKind::SevenZ, &opts, |_| {});
        assert_eq!(r.status, JobStatus::Found, "7z: {}", r.message);
        assert_eq!(r.password.as_deref(), Some("42"));
    }

    // RAR password unrar (custom charset)
    {
        let path = require("sample_rar_crypted.rar");
        let opts = CharsetOptions {
            min_len: 5,
            max_len: 5,
            digits: false,
            lowercase: false,
            uppercase: false,
            symbols: false,
            custom: "unra".into(),
        };
        let r = engine.crack_file(12, path, FileKind::Rar, &opts, |_| {});
        assert_eq!(r.status, JobStatus::Found, "rar: {}", r.message);
        assert_eq!(r.password.as_deref(), Some("unrar"));
    }

    // PDF password 123456 (custom charset of those digits only)
    {
        let path = require("sample_pdf_rc4.pdf");
        let opts = CharsetOptions {
            min_len: 6,
            max_len: 6,
            digits: false,
            lowercase: false,
            uppercase: false,
            symbols: false,
            custom: "123456".into(),
        };
        let r = engine.crack_file(13, path, FileKind::Pdf, &opts, |_| {});
        assert_eq!(r.status, JobStatus::Found, "pdf: {}", r.message);
        assert_eq!(r.password.as_deref(), Some("123456"));
    }

    // Office Standard password Password1234_ — too long for full brute;
    // verify via handler only (covered in matrix). Here we just assert detect+encrypted.
    {
        let path = require("sample_office_enc.docx");
        assert!(OfficeHandler.is_encrypted(&path).unwrap());
        assert!(OfficeHandler
            .try_password(&path, "Password1234_")
            .unwrap());
    }
}

#[test]
fn plain_office_and_detect() {
    let path = require("sample_office_plain.docx");
    assert_eq!(FileKind::detect(&path), FileKind::Office);
    assert!(!OfficeHandler.is_encrypted(&path).unwrap());
}
