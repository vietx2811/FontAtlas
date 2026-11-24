use std::path::Path;
use std::process::Command;

use fontatlas_cep::{Identity, VerifyError, canonical_manifest, canonical_signed_info, sign_package, verify_package};

fn files() -> Vec<(String, Vec<u8>)> {
    vec![
        ("mimetype".into(), b"application/vnd.adobe.air-ucf-package+zip".to_vec()),
        ("CSXS/manifest.xml".into(), b"<m/>".to_vec()),
        ("index.html".into(), b"<html></html>".to_vec()),
        ("jsx/host.jsx".into(), "// tiếng Việt".as_bytes().to_vec()),
    ]
}

fn id() -> Identity {
    static ID: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let dir = std::env::temp_dir().join(format!("fa-cep-test-{}", std::process::id()));
    let i = Identity::load_or_create(&dir).unwrap();
    ID.get_or_init(|| i.cert_der().to_vec());
    i
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> &'a str {
    let i = s.find(a).unwrap() + a.len();
    &s[i..i + s[i..].find(b).unwrap()]
}

#[test]
fn signed_package_verifies_and_detects_every_kind_of_tampering() {
    let id = id();
    let f = files();
    let sig = sign_package(&f, &id).unwrap();
    assert_eq!(verify_package(&sig, &f, &id), Ok(()));

    let mut changed = f.clone();
    changed[2].1 = b"<html>x</html>".to_vec();
    assert_eq!(verify_package(&sig, &changed, &id), Err(VerifyError::FileChanged("index.html".into())));

    let mut extra = f.clone();
    extra.push(("evil.js".into(), b"x".to_vec()));
    assert_eq!(verify_package(&sig, &extra, &id), Err(VerifyError::Unsigned("evil.js".into())));

    let missing: Vec<_> = f.iter().filter(|(p, _)| p != "index.html").cloned().collect();
    assert_eq!(verify_package(&sig, &missing, &id), Err(VerifyError::FileChanged("index.html".into())));

    let other = Identity::generate().unwrap();
    assert_eq!(verify_package(&sig, &f, &other), Err(VerifyError::ForeignCert));

    // sửa chữ ký: đổi một ký tự trong SignatureValue
    let at = sig.find("PackageSignatureValue\">").unwrap() + "PackageSignatureValue\">".len();
    let mut bad = sig.clone().into_bytes();
    bad[at] = if bad[at] == b'A' { b'B' } else { b'A' };
    assert_eq!(verify_package(&String::from_utf8(bad).unwrap(), &f, &id), Err(VerifyError::BadSignature));
}

#[test]
fn signature_verifies_with_openssl_the_way_adobe_would() {
    // Adobe kiểm bằng: RSA-SHA1 của C14N(SignedInfo) với khoá công khai trong X509Certificate.
    let id = id();
    let f = files();
    let sig = sign_package(&f, &id).unwrap();
    let md = between(&sig, "<DigestValue>", "</DigestValue>"); // DigestValue đầu tiên = của Manifest trong SignedInfo
    let dir = tempfile::tempdir().unwrap();
    let p = |n: &str| dir.path().join(n);
    std::fs::write(p("si"), canonical_signed_info(md)).unwrap();
    let b64: String = between(&sig, "PackageSignatureValue\">", "</SignatureValue>").split_whitespace().collect();
    let run = |cmd: &str| Command::new("sh").arg("-c").arg(cmd).current_dir(dir.path()).output().unwrap();
    assert!(run(&format!("printf %s '{b64}' | base64 -d > sig")).status.success());
    let cert: String = between(&sig, "<X509Certificate>", "</X509Certificate>").split_whitespace().collect();
    std::fs::write(p("c.pem"), format!("-----BEGIN CERTIFICATE-----\n{cert}\n-----END CERTIFICATE-----\n")).unwrap();
    assert!(run("openssl x509 -in c.pem -pubkey -noout > pub.pem").status.success());
    let r = run("openssl dgst -sha1 -verify pub.pem -signature sig si");
    assert!(String::from_utf8_lossy(&r.stdout).contains("Verified OK"), "{:?}", r);
    // digest của Manifest khớp bản canonical ta dựng
    let expect = {
        use base64::Engine;
        use sha2::{Digest, Sha256};
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(canonical_manifest(&f).as_bytes()))
    };
    assert_eq!(md, expect);
    // chứng chỉ tự ký: issuer = subject
    let r = run("openssl x509 -in c.pem -noout -issuer -subject");
    let t = String::from_utf8_lossy(&r.stdout).to_string();
    let (iss, sub) = (t.lines().next().unwrap().trim_start_matches("issuer="), t.lines().nth(1).unwrap().trim_start_matches("subject="));
    assert_eq!(iss.trim(), sub.trim(), "{t}");
}

/// Đối chiếu với panel nền THẬT đang có trên máy (Logi Plugin for Illustrator, chứng chỉ tự ký): bộ dựng C14N của ta phải
/// cho đúng DigestValue của Manifest mà công cụ ký của Adobe/hãng đã ghi. Bỏ qua nếu máy không có panel này.
#[test]
fn canonical_manifest_reproduces_a_real_signed_panel() {
    let root = Path::new("/Library/Application Support/Adobe/CEP/extensions/Logi Plugin for Illustrator");
    let Ok(xml) = std::fs::read_to_string(root.join("META-INF/signatures.xml")) else {
        eprintln!("bỏ qua: máy không có panel Logi");
        return;
    };
    let manifest = between(&xml, "<Manifest Id=\"PackageContents\">", "</Manifest>");
    let mut files = vec![];
    let mut rest = manifest;
    while let Some(i) = rest.find("<Reference URI=\"") {
        let uri = between(&rest[i..], "<Reference URI=\"", "\"").to_string();
        files.push((uri.clone(), std::fs::read(root.join(&uri)).unwrap()));
        rest = &rest[i + 20..];
    }
    assert!(files.len() > 5);
    let want = between(&xml, "URI=\"#PackageContents\">", "</Reference>");
    let want = between(want, "<DigestValue>", "</DigestValue>");
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let got = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(canonical_manifest(&files).as_bytes()));
    assert_eq!(got, want, "định dạng manifest của ta lệch bản ký thật");
}

fn panel_file(p: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("panel").join(p)).unwrap()
}

#[test]
fn panel_validates_broker_json_and_scans_without_truncating_runs() {
    let main = panel_file("main.js");
    assert!(main.contains("[0-9a-f]{64}") && main.contains("65535"), "kiểm cổng và token như script InDesign");
    assert_eq!(main.matches("POLL_MS").count(), 3, "chỉ một POLL_MS (20 s) được khai báo và dùng");
    let host = panel_file("jsx/host.jsx");
    assert!(!host.contains("r < 300"), "không cắt 300 text run");
    assert!(host.contains("st.ri"), "vị trí quét có chỉ số run để quét tiếp theo lát");
}

/// Hồi quy cho test chập chờn: nhiều luồng cùng `load_or_create` trên một thư mục MỚI (đúng như `id()` gọi từ hai test song song).
/// Trước khi sửa: các luồng dùng chung tệp tạm `signing.tmp`, luồng đến sau `rename` mất tệp → `Err` (và khoá/chứng chỉ có thể
/// của hai lần sinh khác nhau). Mọi luồng phải thành công và nhận CÙNG một chứng chỉ, trùng với thứ đã lưu trên đĩa.
#[test]
fn concurrent_load_or_create_on_a_fresh_dir_is_safe_and_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity");
    let certs: Vec<Vec<u8>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..8).map(|_| s.spawn(|| Identity::load_or_create(&path).expect("không được lỗi khi chạy song song").cert_der().to_vec())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(certs.iter().all(|c| c == &certs[0]), "mọi luồng phải thấy cùng một danh tính");
    assert_eq!(std::fs::read(path.join("signing.cert.der")).unwrap(), certs[0], "chứng chỉ trên đĩa trùng danh tính đã trả");
    let again = Identity::load_or_create(&path).unwrap();
    assert_eq!(again.cert_der(), &certs[0][..], "lần nạp sau đọc đúng cặp đã lưu");
    let leftovers: Vec<_> = std::fs::read_dir(&path).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.contains("tmp")).collect();
    assert!(leftovers.is_empty(), "không để lại tệp tạm: {leftovers:?}");
}
