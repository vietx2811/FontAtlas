//! B7 (nguyên mẫu): "gói thư viện" — ảnh chụp nhất quán của library.db + user.db kèm manifest, để nhập lại sau khi cài lại máy.
//!
//! Gói là MỘT thư mục (thường đặt cạnh Organized Library):
//! ```text
//! <gói>/manifest.json   phiên bản định dạng, schema hai DB, số liệu, danh sách file + BLAKE3, gốc các nguồn lúc xuất
//! <gói>/library.db      font, face, vị trí, phân tích, embedding (chụp bằng VACUUM INTO: nhất quán, không cần dừng app)
//! <gói>/user.db         Dự án, Collection, Tag, Yêu thích, nguồn, CÀI ĐẶT (bảng settings), trạng thái kích hoạt…
//! ```
//! Cài đặt và embedding đã nằm sẵn trong hai DB nên không có file riêng. File font KHÔNG nằm trong gói (có thể hàng chục GB):
//! chúng ở Organized Library / thư mục nguồn; khi đường dẫn đổi sau cài lại, `import_pack` nhận bảng đổi tiền tố.
//!
//! Nguyên tắc: KHÔNG bao giờ ghi đè dữ liệu có sẵn (đích phải chưa có DB), KHÔNG nhập gói mới hơn app hiểu, KHÔNG nhập gói hỏng
//! (kiểm kích thước + BLAKE3 từng file). Nhập dựng từng file ở tên tạm rồi đổi tên: user.db TRƯỚC, library.db SAU — crash giữa chừng
//! chỉ để lại user.db, lần mở sau library tự dựng rỗng và quét lại, không mất dữ liệu người dùng.
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::open::{LIBRARY_FILE, Store, USER_FILE, now_ms};
use crate::schema::{LIBRARY_VERSION, USER_VERSION};

/// Phiên bản định dạng gói. Tăng khi manifest/bố cục đổi theo cách bản cũ không đọc được.
pub const PACK_FORMAT: u32 = 1;
pub const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackFile {
    pub name: String,
    pub size: u64,
    pub blake3: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PackCounts {
    pub files: u32,
    pub faces: u32,
    pub embeddings: u32,
    pub sources: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    pub format: u32,
    pub app_version: String,
    pub library_schema: u32,
    pub user_schema: u32,
    pub created_at: i64,
    pub counts: PackCounts,
    /// Thư mục gốc của các nguồn lúc xuất (để người nhập biết cần đổi tiền tố nào).
    pub source_roots: Vec<String>,
    pub files: Vec<PackFile>,
}

/// Kết quả nhập.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub counts: PackCounts,
    pub remapped_locations: u32,
    pub remapped_sources: u32,
}

fn hash_file(p: &Path) -> Result<(u64, String), StoreError> {
    let mut h = blake3::Hasher::new();
    let f = std::fs::File::open(p)?;
    let size = f.metadata()?.len();
    h.update_reader(f)?;
    Ok((size, h.finalize().to_hex().to_string()))
}

fn db_version(p: &Path) -> Result<u32, StoreError> {
    let c = rusqlite::Connection::open_with_flags(p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(c.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))?)
}

impl Store {
    /// Xuất gói vào `out` (thư mục chưa tồn tại hoặc rỗng). Chỉ đọc DB đang mở; không đụng file nào của người dùng.
    pub fn export_pack(&self, out: &Path) -> Result<PackManifest, StoreError> {
        if out.exists() && std::fs::read_dir(out)?.next().is_some() {
            return Err(StoreError::Conflict(format!("thư mục gói không rỗng: {}", out.display())));
        }
        std::fs::create_dir_all(out)?;
        let tmp = out.join(".writing");
        std::fs::create_dir_all(&tmp)?;
        let (lib, usr) = (tmp.join(LIBRARY_FILE), tmp.join(USER_FILE));
        // VACUUM INTO: bản chụp nhất quán đã gom WAL, không cần dừng ghi
        self.conn.execute("VACUUM main INTO ?1", [lib.to_string_lossy()])?;
        self.conn.execute("VACUUM u INTO ?1", [usr.to_string_lossy()])?;
        let count = |sql: &str| -> Result<u32, StoreError> { Ok(self.conn.query_row(sql, [], |r| r.get::<_, i64>(0))? as u32) };
        let counts = PackCounts {
            files: count("SELECT count(*) FROM files")?,
            faces: count("SELECT count(*) FROM faces")?,
            embeddings: count("SELECT count(*) FROM embeddings")?,
            sources: count("SELECT count(*) FROM u.sources")?,
        };
        let source_roots: Vec<String> = {
            let mut st = self.conn.prepare("SELECT path FROM u.sources WHERE path IS NOT NULL ORDER BY source_id")?;
            st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        let mut files = Vec::new();
        for name in [LIBRARY_FILE, USER_FILE] {
            let (size, b3) = hash_file(&tmp.join(name))?;
            files.push(PackFile { name: name.into(), size, blake3: b3 });
            std::fs::rename(tmp.join(name), out.join(name))?;
        }
        let m = PackManifest {
            format: PACK_FORMAT,
            app_version: env!("CARGO_PKG_VERSION").into(),
            library_schema: db_version(&out.join(LIBRARY_FILE))?,
            user_schema: db_version(&out.join(USER_FILE))?,
            created_at: now_ms(),
            counts,
            source_roots,
            files,
        };
        // manifest ghi SAU CÙNG và qua file tạm: có manifest = gói trọn vẹn
        let mt = tmp.join(MANIFEST_FILE);
        std::fs::write(&mt, serde_json::to_vec_pretty(&m).map_err(|e| StoreError::Invalid(e.to_string()))?)?;
        std::fs::rename(&mt, out.join(MANIFEST_FILE))?;
        let _ = std::fs::remove_dir(&tmp);
        Ok(m)
    }
}

/// Đọc và kiểm gói: manifest hợp lệ, định dạng/schema không mới hơn app, đủ file, đúng kích thước và BLAKE3.
pub fn verify_pack(pack: &Path) -> Result<PackManifest, StoreError> {
    let raw = std::fs::read(pack.join(MANIFEST_FILE))
        .map_err(|e| StoreError::Invalid(format!("không đọc được manifest của gói ({e}); gói chưa xuất xong hoặc không phải gói thư viện")))?;
    let m: PackManifest = serde_json::from_slice(&raw).map_err(|e| StoreError::Invalid(format!("manifest hỏng: {e}")))?;
    if m.format > PACK_FORMAT {
        return Err(StoreError::NewerSchema { db: "pack", found: m.format, supported: PACK_FORMAT });
    }
    if m.library_schema > LIBRARY_VERSION {
        return Err(StoreError::NewerSchema { db: "library", found: m.library_schema, supported: LIBRARY_VERSION });
    }
    if m.user_schema > USER_VERSION {
        return Err(StoreError::NewerSchema { db: "user", found: m.user_schema, supported: USER_VERSION });
    }
    for need in [LIBRARY_FILE, USER_FILE] {
        let f = m.files.iter().find(|f| f.name == need).ok_or_else(|| StoreError::Invalid(format!("gói thiếu {need}")))?;
        let (size, b3) = hash_file(&pack.join(&f.name)).map_err(|_| StoreError::Invalid(format!("gói thiếu file {need}")))?;
        if size != f.size || b3 != f.blake3 {
            return Err(StoreError::Invalid(format!("{need} trong gói đã hỏng (sai kích thước hoặc BLAKE3)")));
        }
    }
    Ok(m)
}

/// Nhập gói vào `data_dir` (thư mục dữ liệu app, CHƯA có library.db/user.db). `remap`: (tiền tố cũ → tiền tố mới) áp lên mọi đường
/// dẫn vị trí file và gốc nguồn, theo thứ tự, tiền tố đầu khớp thì dùng. Sau khi nhập nên quét lại các nguồn để xác nhận file còn đó.
pub fn import_pack(pack: &Path, data_dir: &Path, remap: &[(String, String)]) -> Result<ImportReport, StoreError> {
    let m = verify_pack(pack)?;
    for f in [LIBRARY_FILE, USER_FILE] {
        if data_dir.join(f).exists() {
            return Err(StoreError::Conflict(format!("đã có {f} ở {}: không ghi đè dữ liệu có sẵn", data_dir.display())));
        }
    }
    std::fs::create_dir_all(data_dir)?;
    let stage: PathBuf = data_dir.join(".import-staging");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage)?;
    for f in [USER_FILE, LIBRARY_FILE] {
        std::fs::copy(pack.join(f), stage.join(f))?;
    }
    // đổi tiền tố trên bản đã dàn (chưa phải DB chính thức), rồi đổi tên: user.db trước, library.db sau
    let mut rep = ImportReport { counts: m.counts, ..Default::default() };
    {
        let c = rusqlite::Connection::open(stage.join(LIBRARY_FILE))?;
        for (old, new) in remap {
            rep.remapped_locations += c.execute(
                "UPDATE file_locations SET path = ?2 || substr(path, length(?1) + 1) WHERE substr(path, 1, length(?1)) = ?1",
                params![old, new],
            )? as u32;
        }
        let u = rusqlite::Connection::open(stage.join(USER_FILE))?;
        for (old, new) in remap {
            rep.remapped_sources += u.execute(
                "UPDATE sources SET path = ?2 || substr(path, length(?1) + 1) WHERE path IS NOT NULL AND substr(path, 1, length(?1)) = ?1",
                params![old, new],
            )? as u32;
        }
        c.execute_batch("PRAGMA journal_mode=DELETE;")?;
        u.execute_batch("PRAGMA journal_mode=DELETE;")?;
    }
    for f in [USER_FILE, LIBRARY_FILE] {
        std::fs::rename(stage.join(f), data_dir.join(f))?;
    }
    let _ = std::fs::remove_dir(&stage);
    Ok(rep)
}

/// Khôi phục (cài lại máy / thư viện hỏng): như [`import_pack`] nhưng nếu `data_dir` đã có DB (vd lần mở đầu sau cài đã tạo DB rỗng) thì
/// DỜI chúng sang `pre-restore-<thời điểm>/` trong cùng thư mục (không xoá) rồi mới nhập. Gói được kiểm TRƯỚC khi dời bất cứ thứ gì.
/// Chỉ gọi khi KHÔNG có kết nối nào đang mở các DB đó (app gọi lúc khởi động, trước `Store::open`).
/// Trả (báo cáo, thư mục sao lưu nếu có).
pub fn restore_pack(pack: &Path, data_dir: &Path, remap: &[(String, String)]) -> Result<(ImportReport, Option<PathBuf>), StoreError> {
    verify_pack(pack)?;
    let mut backup = None;
    let has = |f: &str| data_dir.join(f).exists();
    if has(LIBRARY_FILE) || has(USER_FILE) {
        let b = data_dir.join(format!("pre-restore-{}", now_ms()));
        std::fs::create_dir_all(&b)?;
        for base in [LIBRARY_FILE, USER_FILE] {
            for ext in ["", "-wal", "-shm"] {
                let name = format!("{base}{ext}");
                if data_dir.join(&name).exists() {
                    std::fs::rename(data_dir.join(&name), b.join(&name))?;
                }
            }
        }
        backup = Some(b);
    }
    let rep = import_pack(pack, data_dir, remap)?;
    Ok((rep, backup))
}
