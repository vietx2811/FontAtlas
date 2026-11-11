//! Ký và kiểm gói theo định dạng ZXP. Đối chiếu với panel thật của Logi (xem tests/format.rs).
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::Identity;

const DSIG: &str = "http://www.w3.org/2000/09/xmldsig#";
const C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
const SHA256: &str = "http://www.w3.org/2001/04/xmlenc#sha256";

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("ký: {0}")]
    Sign(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("không có hoặc không đọc được META-INF/signatures.xml")]
    NoSignature,
    #[error("chứng chỉ trong gói không phải của máy này")]
    ForeignCert,
    #[error("tệp {0} đã đổi hoặc mất")]
    FileChanged(String),
    #[error("có tệp lạ không được ký: {0}")]
    Unsigned(String),
    #[error("chữ ký không đúng")]
    BadSignature,
}

fn digest_b64(data: &[u8]) -> String {
    B64.encode(Sha256::digest(data))
}

fn wrap64(s: &str) -> String {
    s.as_bytes().chunks(64).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join("\n")
}

/// `files`: (đường dẫn tương đối, nội dung), KHÔNG gồm `META-INF/`.
fn ordered(files: &[(String, Vec<u8>)]) -> Vec<&(String, Vec<u8>)> {
    let mut v: Vec<_> = files.iter().collect();
    // Cùng thứ tự công cụ ký của Adobe: mimetype đầu, rồi theo tên không phân biệt hoa thường (đối chiếu tests/format.rs).
    v.sort_by_key(|(p, _)| (p != "mimetype", p.to_ascii_lowercase(), p.clone()));
    v
}

fn manifest_refs(files: &[(String, Vec<u8>)], canonical: bool) -> String {
    let dm = if canonical {
        format!("<DigestMethod Algorithm=\"{SHA256}\"></DigestMethod>")
    } else {
        format!("<DigestMethod Algorithm=\"{SHA256}\"/>")
    };
    ordered(files)
        .iter()
        .map(|(p, d)| format!("<Reference URI=\"{p}\">{dm}<DigestValue>{}</DigestValue></Reference>", digest_b64(d)))
        .collect()
}

/// Dạng C14N (inclusive, 20010315) của `<Manifest>`: namespace kế thừa từ `<Signature>`, thẻ rỗng viết đầy đủ.
pub fn canonical_manifest(files: &[(String, Vec<u8>)]) -> String {
    format!("<Manifest xmlns=\"{DSIG}\" Id=\"PackageContents\">\n{}</Manifest>", manifest_refs(files, true))
}

/// Dạng C14N của `<SignedInfo>` (đây là thứ được ký).
pub fn canonical_signed_info(manifest_digest: &str) -> String {
    format!(
        "<SignedInfo xmlns=\"{DSIG}\">\n<CanonicalizationMethod Algorithm=\"{C14N}\"></CanonicalizationMethod>\n<SignatureMethod Algorithm=\"{DSIG}rsa-sha1\"></SignatureMethod>\n<Reference Type=\"{DSIG}Manifest\" URI=\"#PackageContents\">\n<Transforms>\n<Transform Algorithm=\"{C14N}\"></Transform>\n</Transforms>\n<DigestMethod Algorithm=\"{SHA256}\"></DigestMethod>\n<DigestValue>{manifest_digest}</DigestValue>\n</Reference>\n</SignedInfo>"
    )
}

fn plain_signed_info(manifest_digest: &str) -> String {
    format!(
        "<SignedInfo>\n<CanonicalizationMethod Algorithm=\"{C14N}\"/>\n<SignatureMethod Algorithm=\"{DSIG}rsa-sha1\"/>\n<Reference Type=\"{DSIG}Manifest\" URI=\"#PackageContents\">\n<Transforms>\n<Transform Algorithm=\"{C14N}\"/>\n</Transforms>\n<DigestMethod Algorithm=\"{SHA256}\"/>\n<DigestValue>{manifest_digest}</DigestValue>\n</Reference>\n</SignedInfo>"
    )
}

/// Nội dung `META-INF/signatures.xml` cho tập tệp này.
pub fn sign_package(files: &[(String, Vec<u8>)], id: &Identity) -> Result<String, SignError> {
    let manifest_digest = digest_b64(canonical_manifest(files).as_bytes());
    let sk = SigningKey::<Sha1>::new(id.key.clone());
    let sig = sk.try_sign(canonical_signed_info(&manifest_digest).as_bytes()).map_err(|e| SignError::Sign(e.to_string()))?;
    let manifest = format!("<Manifest Id=\"PackageContents\">\n{}</Manifest>", manifest_refs(files, false));
    Ok(format!(
        "<signatures>\n<Signature xmlns=\"{DSIG}\" Id=\"PackageSignature\">\n{}\n<SignatureValue Id=\"PackageSignatureValue\">{}</SignatureValue>\n\n<KeyInfo>\n<X509Data>\n<X509Certificate>{}</X509Certificate>\n</X509Data>\n</KeyInfo>\n<Object>\n{manifest}\n</Object>\n</Signature>\n</signatures>",
        plain_signed_info(&manifest_digest),
        wrap64(&B64.encode(sig.to_bytes())),
        wrap64(&B64.encode(&id.cert_der)),
    ))
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some(&s[a..b])
}

/// Kiểm một gói đã cài: chữ ký là của khoá máy này, mọi tệp khớp digest, không có tệp lạ ngoài `META-INF/`.
/// `files` = nội dung hiện có trên đĩa (không gồm `META-INF/`).
pub fn verify_package(signatures_xml: &str, files: &[(String, Vec<u8>)], id: &Identity) -> Result<(), VerifyError> {
    let cert = between(signatures_xml, "<X509Certificate>", "</X509Certificate>").ok_or(VerifyError::NoSignature)?;
    let cert: String = cert.split_whitespace().collect();
    if B64.decode(cert).ok().as_deref() != Some(id.cert_der.as_slice()) {
        return Err(VerifyError::ForeignCert);
    }
    let manifest = between(signatures_xml, "<Manifest Id=\"PackageContents\">", "</Manifest>").ok_or(VerifyError::NoSignature)?;
    let mut listed = vec![];
    let mut rest = manifest;
    while let Some(uri) = between(rest, "<Reference URI=\"", "\"") {
        let dv = between(rest, "<DigestValue>", "</DigestValue>").ok_or(VerifyError::NoSignature)?;
        listed.push((uri.to_string(), dv.to_string()));
        rest = &rest[rest.find("</Reference>").ok_or(VerifyError::NoSignature)? + "</Reference>".len()..];
    }
    for (uri, dv) in &listed {
        match files.iter().find(|(p, _)| p == uri) {
            Some((_, d)) if digest_b64(d) == *dv => {}
            _ => return Err(VerifyError::FileChanged(uri.clone())),
        }
    }
    if let Some((p, _)) = files.iter().find(|(p, _)| !listed.iter().any(|(u, _)| u == p)) {
        return Err(VerifyError::Unsigned(p.clone()));
    }
    // chữ ký trên SignedInfo dựng lại từ digest của manifest dựng lại
    let md = digest_b64(canonical_manifest(files).as_bytes());
    let sv: String = between(signatures_xml, "<SignatureValue Id=\"PackageSignatureValue\">", "</SignatureValue>")
        .ok_or(VerifyError::NoSignature)?
        .split_whitespace()
        .collect();
    let raw = B64.decode(sv).map_err(|_| VerifyError::BadSignature)?;
    let sig = Signature::try_from(raw.as_slice()).map_err(|_| VerifyError::BadSignature)?;
    VerifyingKey::<Sha1>::new(id.key.to_public_key())
        .verify(canonical_signed_info(&md).as_bytes(), &sig)
        .map_err(|_| VerifyError::BadSignature)
}
