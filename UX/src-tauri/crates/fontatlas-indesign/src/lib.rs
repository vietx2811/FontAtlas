//! fontatlas-indesign: startup script của Font Atlas cho InDesign và InCopy.
//!
//! InDesign chạy mọi script trong `~/Library/Preferences/Adobe InDesign/Version <N>.0/<locale>/Scripts/startup scripts/`
//! lúc khởi động (InCopy: `Adobe InCopy`). Script không cần chữ ký, không cần Automation. Ta chỉ cài vào các phiên bản
//! và locale ĐÃ có thư mục (app đã chạy ít nhất một lần): không đoán locale cho bản chưa chạy.
use std::path::PathBuf;

use sha2::{Digest, Sha256};

pub const SCRIPT_NAME: &str = "FontAtlas_Startup.jsx";
const TEMPLATE: &str = include_str!("../script/FontAtlas_Startup.jsx");
/// Dòng đầu của script do ta cài: dùng để nhận ra tệp của mình (không bao giờ xoá/đè tệp cùng tên của người khác).
const MARK: &str = "// FontAtlas startup script ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Product {
    InDesign,
    InCopy,
}

impl Product {
    pub const ALL: [Product; 2] = [Product::InDesign, Product::InCopy];
    fn dir_name(self) -> &'static str {
        match self {
            Product::InDesign => "Adobe InDesign",
            Product::InCopy => "Adobe InCopy",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Product::InDesign => "InDesign",
            Product::InCopy => "InCopy",
        }
    }
}

/// Một nơi script được/sẽ được cài: `…/Version 21.0/en_US/Scripts/startup scripts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub product: Product,
    /// "21.0"
    pub version: String,
    pub locale: String,
    pub dir: PathBuf,
}

impl Target {
    pub fn script_path(&self) -> PathBuf {
        self.dir.join(SCRIPT_NAME)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetStatus {
    NotInstalled,
    Installed,
    /// Có script của ta nhưng khác bản hiện tại (phiên bản, đường dẫn broker.json, hoặc bị sửa).
    Stale,
    /// Có tệp cùng tên nhưng không phải của Font Atlas.
    Foreign,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("tệp {0} đã có và không phải của Font Atlas")]
    Foreign(PathBuf),
}

pub struct Report {
    pub target: Target,
    pub result: Result<TargetStatus, String>,
}

pub struct StartupInstaller {
    prefs_root: PathBuf,
    broker_file: PathBuf,
    version: String,
}

/// `~/Library/Preferences`.
pub fn default_prefs_root() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Preferences"))
}

fn js_string(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => o.push(' '),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl StartupInstaller {
    pub fn new(prefs_root: impl Into<PathBuf>, broker_file: impl Into<PathBuf>, version: &str) -> Self {
        Self { prefs_root: prefs_root.into(), broker_file: broker_file.into(), version: version.into() }
    }

    /// Nội dung script sẽ cài.
    pub fn script_text(&self) -> String {
        TEMPLATE.replace("@VERSION@", &self.version).replace("@BROKER_JSON_PATH@", &js_string(&self.broker_file.to_string_lossy()))
    }

    /// Các bản InDesign/InCopy đã có thư mục locale (đã chạy ít nhất một lần).
    pub fn targets(&self) -> Vec<Target> {
        let mut out = vec![];
        for product in Product::ALL {
            let root = self.prefs_root.join(product.dir_name());
            let Ok(versions) = std::fs::read_dir(&root) else { continue };
            for v in versions.flatten() {
                let name = v.file_name().to_string_lossy().into_owned();
                let Some(version) = name.strip_prefix("Version ") else { continue };
                if !v.path().is_dir() {
                    continue;
                }
                let Ok(locales) = std::fs::read_dir(v.path()) else { continue };
                for l in locales.flatten() {
                    // thư mục locale có dạng en_US, vi_VN, ... và chứa `Scripts` hoặc ít nhất là thư mục thật
                    let lname = l.file_name().to_string_lossy().into_owned();
                    if !l.path().is_dir() || !is_locale(&lname) {
                        continue;
                    }
                    out.push(Target {
                        product,
                        version: version.to_string(),
                        locale: lname,
                        dir: l.path().join("Scripts").join("startup scripts"),
                    });
                }
            }
        }
        out.sort_by(|a, b| (a.product as u8, &a.version, &a.locale).cmp(&(b.product as u8, &b.version, &b.locale)));
        out
    }

    pub fn target_status(&self, t: &Target) -> TargetStatus {
        let Ok(cur) = std::fs::read_to_string(t.script_path()) else {
            return if t.script_path().exists() { TargetStatus::Foreign } else { TargetStatus::NotInstalled };
        };
        if !cur.starts_with(MARK) {
            return TargetStatus::Foreign;
        }
        if hash(cur.as_bytes()) == hash(self.script_text().as_bytes()) { TargetStatus::Installed } else { TargetStatus::Stale }
    }

    pub fn status(&self) -> Vec<(Target, TargetStatus)> {
        self.targets().into_iter().map(|t| { let s = self.target_status(&t); (t, s) }).collect()
    }

    /// Cài vào mọi nơi tìm được. Mỗi nơi báo riêng (một nơi lỗi không chặn nơi khác); tệp lạ cùng tên không bị đè.
    pub fn install(&self) -> Vec<Report> {
        let text = self.script_text();
        self.targets()
            .into_iter()
            .map(|t| {
                let result = self.install_one(&t, &text).map_err(|e| e.to_string());
                Report { target: t, result }
            })
            .collect()
    }

    fn install_one(&self, t: &Target, text: &str) -> Result<TargetStatus, Error> {
        let p = t.script_path();
        if p.exists() && self.target_status(t) == TargetStatus::Foreign {
            return Err(Error::Foreign(p));
        }
        std::fs::create_dir_all(&t.dir)?;
        let tmp = t.dir.join(format!(".{SCRIPT_NAME}.tmp-{}", std::process::id()));
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &p)?;
        Ok(TargetStatus::Installed)
    }

    /// Gỡ script của ta ở mọi nơi. Trả số tệp đã xoá. Tệp lạ cùng tên được giữ nguyên.
    pub fn remove(&self) -> Result<usize, Error> {
        let mut n = 0;
        for t in self.targets() {
            match self.target_status(&t) {
                TargetStatus::Installed | TargetStatus::Stale => {
                    std::fs::remove_file(t.script_path())?;
                    n += 1;
                }
                _ => {}
            }
        }
        Ok(n)
    }
}

fn is_locale(s: &str) -> bool {
    let b = s.as_bytes();
    // en_US, vi_VN, zh_CN, ... (không phải "Scripts Panel", "Presets"...)
    b.len() == 5 && b[2] == b'_' && b[..2].iter().all(u8::is_ascii_lowercase) && b[3..].iter().all(u8::is_ascii_uppercase)
}

fn hash(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

