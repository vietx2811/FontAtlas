use std::path::{Path, PathBuf};

use crate::{Identity, IdentityError, SignError, VerifyError, sign_package, verify_package};

pub const BUNDLE_ID: &str = "com.fontatlas.autoactivate";

const MIMETYPE: &[u8] = include_bytes!("../panel/mimetype");
const MANIFEST: &str = include_str!("../panel/CSXS/manifest.xml");
const INDEX: &[u8] = include_bytes!("../panel/index.html");
const MAIN_JS: &[u8] = include_bytes!("../panel/main.js");
const HOST_JSX: &[u8] = include_bytes!("../panel/jsx/host.jsx");

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("thư mục extensions của Adobe: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Sign(#[from] SignError),
    /// Đã có thư mục cùng tên nhưng không phải của Font Atlas: không đè, không xoá.
    #[error("thư mục {0} đã có và không phải của Font Atlas")]
    Foreign(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelStatus {
    NotInstalled,
    /// Đã cài, chữ ký đúng, cùng phiên bản và cùng broker.json.
    Installed { version: String },
    /// Đã cài nhưng cần cài lại (khác phiên bản, khác broker.json, chữ ký hỏng hoặc không phải khoá máy này).
    Stale { reason: String },
    Foreign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub dir: PathBuf,
    pub replaced: bool,
    /// Illustrator đọc manifest lúc khởi động: đang chạy thì phải khởi động lại mới thấy panel.
    pub restart_illustrator: bool,
}

/// `~/Library/Application Support/Adobe/CEP/extensions` của người dùng hiện tại.
pub fn default_extensions_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Adobe/CEP/extensions"))
}

pub struct PanelInstaller {
    extensions_dir: PathBuf,
    state_dir: PathBuf,
    broker_file: PathBuf,
    version: String,
}

impl PanelInstaller {
    /// `state_dir`: nơi giữ khoá ký của máy (thư mục dữ liệu của app). `broker_file`: đường dẫn tuyệt đối `broker.json`.
    pub fn new(extensions_dir: impl Into<PathBuf>, state_dir: impl Into<PathBuf>, broker_file: impl Into<PathBuf>, version: &str) -> Self {
        Self { extensions_dir: extensions_dir.into(), state_dir: state_dir.into(), broker_file: broker_file.into(), version: version.into() }
    }

    pub fn panel_dir(&self) -> PathBuf {
        self.extensions_dir.join(BUNDLE_ID)
    }

    fn files(&self) -> Vec<(String, Vec<u8>)> {
        let cfg = serde_json::json!({ "broker": self.broker_file.to_string_lossy() });
        vec![
            ("mimetype".into(), MIMETYPE.to_vec()),
            ("CSXS/manifest.xml".into(), MANIFEST.replace("@VERSION@", &self.version).into_bytes()),
            ("index.html".into(), INDEX.to_vec()),
            ("main.js".into(), MAIN_JS.to_vec()),
            ("jsx/host.jsx".into(), HOST_JSX.to_vec()),
            ("fontatlas.json".into(), serde_json::to_vec_pretty(&cfg).unwrap_or_default()),
        ]
    }

    pub fn install(&self) -> Result<InstallReport, InstallError> {
        let dir = self.panel_dir();
        let existed = dir.exists();
        if existed && !is_ours(&dir) {
            return Err(InstallError::Foreign(dir));
        }
        let id = Identity::load_or_create(&self.state_dir)?;
        let files = self.files();
        let sig = sign_package(&files, &id)?;
        std::fs::create_dir_all(&self.extensions_dir)?;
        self.sweep_leftovers();
        let tmp = self.extensions_dir.join(format!(".{BUNDLE_ID}.tmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        for (p, data) in &files {
            let f = tmp.join(p);
            if let Some(parent) = f.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(f, data)?;
        }
        std::fs::create_dir_all(tmp.join("META-INF"))?;
        std::fs::write(tmp.join("META-INF/signatures.xml"), sig)?;
        // Thay nguyên tử: đưa bản cũ sang chỗ tạm TRƯỚC, đặt bản mới vào, xoá bản cũ sau cùng. Lỗi giữa chừng thì trả bản cũ về,
        // không bao giờ để Illustrator thấy một thư mục panel thiếu/dở.
        let old = self.extensions_dir.join(format!(".{BUNDLE_ID}.old-{}", std::process::id()));
        if existed {
            std::fs::rename(&dir, &old)?;
        }
        if let Err(e) = std::fs::rename(&tmp, &dir) {
            if existed {
                let _ = std::fs::rename(&old, &dir);
            }
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(e.into());
        }
        if existed {
            let _ = std::fs::remove_dir_all(&old);
        }
        Ok(InstallReport { dir, replaced: existed, restart_illustrator: true })
    }

    /// Dọn thư mục tạm (`.com.fontatlas.autoactivate.tmp-<pid>` / `.old-<pid>`) do lần cài bị ngắt để lại. Chỉ đụng đúng tên của ta.
    fn sweep_leftovers(&self) {
        let Ok(rd) = std::fs::read_dir(&self.extensions_dir) else { return };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(&format!(".{BUNDLE_ID}.tmp-")) || n.starts_with(&format!(".{BUNDLE_ID}.old-")) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }

    /// Khoá ký của máy này đang được giữ ở `state_dir` (để cài lại không phải sinh khoá mới).
    pub fn key_present(&self) -> bool {
        self.state_dir.join("signing.pem").is_file()
    }

    /// Xoá khoá + chứng chỉ ký. Từ chối khi panel còn cài (nó sẽ thành `Stale` vì chữ ký không còn khoá để kiểm): gỡ panel trước.
    pub fn forget_key(&self) -> Result<bool, InstallError> {
        if self.panel_dir().exists() {
            return Err(InstallError::Io(std::io::Error::other("gỡ panel trước khi xoá khoá ký")));
        }
        let mut any = false;
        for f in ["signing.pem", "signing.cert.der"] {
            match std::fs::remove_file(self.state_dir.join(f)) {
                Ok(()) => any = true,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(any)
    }

    /// Gỡ panel. `Ok(false)` nếu không có gì để gỡ. Không bao giờ xoá thư mục không phải của Font Atlas.
    pub fn remove(&self) -> Result<bool, InstallError> {
        self.sweep_leftovers();
        let dir = self.panel_dir();
        if !dir.exists() {
            return Ok(false);
        }
        if !is_ours(&dir) {
            return Err(InstallError::Foreign(dir));
        }
        std::fs::remove_dir_all(&dir)?;
        Ok(true)
    }

    pub fn status(&self) -> PanelStatus {
        let dir = self.panel_dir();
        if !dir.exists() {
            return PanelStatus::NotInstalled;
        }
        if !is_ours(&dir) {
            return PanelStatus::Foreign;
        }
        let stale = |r: &str| PanelStatus::Stale { reason: r.into() };
        let Ok(on_disk) = read_tree(&dir) else { return stale("không đọc được thư mục panel") };
        let Ok(sig) = std::fs::read_to_string(dir.join("META-INF/signatures.xml")) else { return stale("thiếu chữ ký") };
        let Ok(id) = Identity::load_or_create(&self.state_dir) else { return stale("không đọc được khoá ký") };
        match verify_package(&sig, &on_disk, &id) {
            Ok(()) => {}
            Err(VerifyError::ForeignCert) => return stale("chữ ký của máy/khoá khác"),
            Err(e) => return stale(&e.to_string()),
        }
        let want = self.files();
        let same = |name: &str| on_disk.iter().find(|(p, _)| p == name).map(|(_, d)| d) == want.iter().find(|(p, _)| p == name).map(|(_, d)| d);
        if !same("CSXS/manifest.xml") {
            return stale("khác phiên bản");
        }
        if !same("fontatlas.json") {
            return stale("broker.json ở chỗ khác");
        }
        if !(same("main.js") && same("jsx/host.jsx") && same("index.html")) {
            return stale("nội dung panel cũ");
        }
        PanelStatus::Installed { version: self.version.clone() }
    }
}

/// Thư mục này là của ta khi manifest khai đúng `ExtensionBundleId`.
fn is_ours(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("CSXS/manifest.xml")).is_ok_and(|m| m.contains(&format!("ExtensionBundleId=\"{BUNDLE_ID}\"")))
}

/// Mọi tệp (đệ quy) trừ `META-INF/`, đường dẫn tương đối.
fn read_tree(root: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            let rel = p.strip_prefix(root).map(|r| r.to_string_lossy().into_owned()).unwrap_or_default();
            if e.file_type()?.is_dir() {
                if rel != "META-INF" {
                    walk(root, &p, out)?;
                }
            } else {
                out.push((rel, std::fs::read(&p)?));
            }
        }
        Ok(())
    }
    let mut v = vec![];
    walk(root, root, &mut v)?;
    Ok(v)
}
