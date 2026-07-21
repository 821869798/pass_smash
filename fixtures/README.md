# Test fixtures

These files are used by `cargo test --bin pass_smash`.

| File | Format | Password | Notes |
|------|--------|----------|-------|
| `sample_zipcrypto.zip` | ZIP ZipCrypto | `42` | Engine brute-force sample |
| `sample_aes.zip` | ZIP AES-256 | `42` | Engine brute-force sample |
| `sample_6digit.zip` | ZIP ZipCrypto | `123456` | 6-digit direct try |
| `sample_6digit_aes.zip` | ZIP AES-256 | `123456` | 6-digit AES |
| `sample_6digit_early.zip` | ZIP ZipCrypto | `000042` | Early hit in 000000–999999 |
| `sample_7z.7z` | 7z AES | `42` | Header encryption (`-mhe=on`) |
| `sample_7z_aes.7z` | 7z AES | `iBlm8NTigvru0Jr0` | From sevenz-rust2 test suite |
| `sample_rar_crypted.rar` | RAR | `unrar` | From unrar crate test data |
| `sample_pdf_rc4.pdf` | PDF V2/RC4 | `123456` | Generated via pypdf |
| `sample_pdf_aes128.pdf` | PDF AES-128 | `654321` | Generated via pypdf |
| `sample_office_plain.docx` | DOCX (no password) | _(none)_ | Plain OOXML ZIP |
| `sample_office_enc.docx` | DOCX Standard | `Password1234_` | From office-crypto tests |
| `sample_office_agile.docx` | DOCX Agile SHA512 | `testPassword` | From office-crypto tests |

## Regenerate (Windows, with 7-Zip CLI)

```bat
echo payload> fixtures\payload.txt
7z a -tzip -p42 -mem=ZipCrypto fixtures\sample_zipcrypto.zip fixtures\payload.txt -y
7z a -tzip -p42 -mem=AES256 fixtures\sample_aes.zip fixtures\payload.txt -y
7z a -t7z -p42 -mhe=on fixtures\sample_7z.7z fixtures\payload.txt -y
```

PDF / Office / RAR samples are taken from upstream test suites (see table).
