use std::path::Path;

use rsa::RsaPrivateKey;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("khoá RSA: {0}")]
    Rsa(String),
    #[error("chứng chỉ: {0}")]
    Cert(String),
    #[error("đọc/ghi khoá: {0}")]
    Io(#[from] std::io::Error),
}

/// Khoá + chứng chỉ tự ký của MỘT máy. Sinh lần đầu khi cài, lưu `signing.pem` (0600) + `signing.cert.der` cạnh nhau.
pub struct Identity {
    pub(crate) key: RsaPrivateKey,
    pub(crate) cert_der: Vec<u8>,
}

impl Identity {
    pub fn generate() -> Result<Self, IdentityError> {
        let key = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).map_err(|e| IdentityError::Rsa(e.to_string()))?;
        let cert_der = self_signed_cert(&key)?;
        Ok(Self { key, cert_der })
    }

    /// Đọc khoá đã lưu trong `dir`, hoặc sinh mới và lưu. Khoá/chứng chỉ hỏng → sinh lại (gói cũ sẽ báo cần cài lại).
    pub fn load_or_create(dir: &Path) -> Result<Self, IdentityError> {
        let (kp, cp) = (dir.join("signing.pem"), dir.join("signing.cert.der"));
        if let (Ok(pem), Ok(cert)) = (std::fs::read_to_string(&kp), std::fs::read(&cp))
            && let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem)
            && !cert.is_empty()
        {
            return Ok(Self { key, cert_der: cert });
        }
        let id = Self::generate()?;
        std::fs::create_dir_all(dir)?;
        write_private(&kp, id.key.to_pkcs8_pem(LineEnding::LF).map_err(|e| IdentityError::Rsa(e.to_string()))?.as_bytes())?;
        std::fs::write(&cp, &id.cert_der)?;
        Ok(id)
    }

    pub fn cert_der(&self) -> &[u8] {
        &self.cert_der
    }
}

fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
    f.write_all(data)?;
    f.sync_all()?;
    std::fs::rename(tmp, path)
}

fn self_signed_cert(key: &RsaPrivateKey) -> Result<Vec<u8>, IdentityError> {
    let pem = key.to_pkcs8_pem(LineEnding::LF).map_err(|e| IdentityError::Rsa(e.to_string()))?;
    let kp = rcgen::KeyPair::from_pem_and_sign_algo(&pem, &rcgen::PKCS_RSA_SHA256).map_err(|e| IdentityError::Cert(e.to_string()))?;
    let mut p = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| IdentityError::Cert(e.to_string()))?;
    p.distinguished_name.push(rcgen::DnType::CommonName, "Font Atlas panel (this Mac)");
    p.distinguished_name.push(rcgen::DnType::OrganizationName, "Font Atlas");
    p.not_before = rcgen::date_time_ymd(2024, 1, 1);
    p.not_after = rcgen::date_time_ymd(2060, 1, 1);
    let cert = p.self_signed(&kp).map_err(|e| IdentityError::Cert(e.to_string()))?;
    Ok(cert.der().to_vec())
}
