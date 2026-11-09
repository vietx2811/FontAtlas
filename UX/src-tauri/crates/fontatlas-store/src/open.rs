//! Mở hai file DB: kiểm phiên bản, migration, sao lưu, dựng lại library hỏng.
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, ErrorCode, OpenFlags};

use crate::error::StoreError;
use crate::schema::{LIBRARY_MIGRATIONS, USER_MIGRATIONS, USER_VERSION, migrate, user_version};

pub const LIBRARY_FILE: &str = "library.db";
pub const USER_FILE: &str = "user.db";
pub const BACKUP_DIR: &str = "backups";
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(5000);

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Một kết nối: library.db là `main`, user.db attach thành `u`.
pub struct Store {
    pub(crate) conn: Connection,
    dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct OpenReport {
    pub library_recreated: bool,
    pub user_backup: Option<PathBuf>,
    /// Số override phân loại `display` bị bỏ khi lên user.db v5 (Display không còn là class).
    pub dropped_display_overrides: u32,
}

impl Store {
    pub fn open(dir: &Path) -> Result<(Store, OpenReport), StoreError> {
        std::fs::create_dir_all(dir)?;
        let user_path = dir.join(USER_FILE);
        let lib_path = dir.join(LIBRARY_FILE);
        let mut rep = OpenReport::default();

        // user.db trước (quý hơn). Kiểm phiên bản bằng kết nối chỉ đọc để không đổi byte nào.
        if user_path.exists() {
            let ro = Connection::open_with_flags(&user_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let found = user_version(&ro)?;
            if found > USER_VERSION {
                return Err(StoreError::NewerSchema { db: "user", found, supported: USER_VERSION });
            }
            // v4 có bảng overrides, v5 loại 'display': đếm trước khi migration xoá
            if found == 4 {
                rep.dropped_display_overrides = ro
                    .query_row("SELECT count(*) FROM classification_overrides WHERE class = 'display'", [], |r| r.get::<_, u32>(0))
                    .unwrap_or(0);
                if rep.dropped_display_overrides > 0 {
                    eprintln!("user.db v4→v5: bỏ {} override phân loại 'display' (Display là tag usage, không phải class)", rep.dropped_display_overrides);
                }
            }
        }
        {
            let uc = Connection::open(&user_path)?;
            uc.busy_timeout(BUSY_TIMEOUT)?;
            rep.user_backup = migrate(&uc, "user", USER_MIGRATIONS, Some(&dir.join(BACKUP_DIR)))?;
            uc.execute_batch("PRAGMA journal_mode=WAL;")?;
        }

        let conn = match open_library(&lib_path, &dir.join(BACKUP_DIR)) {
            Ok(c) => c,
            Err(e) if library_is_broken(&e) => {
                move_aside(&lib_path)?;
                rep.library_recreated = true;
                open_library(&lib_path, &dir.join(BACKUP_DIR))?
            }
            Err(e) => return Err(e),
        };
        conn.execute("ATTACH DATABASE ?1 AS u", [user_path.to_string_lossy()])?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
             PRAGMA main.synchronous=NORMAL; PRAGMA u.synchronous=NORMAL;",
        )?;
        Ok((Store { conn, dir: dir.to_path_buf() }, rep))
    }

    /// Kết nối ghi chỉ mở library.db, KHÔNG attach user.db: `BEGIN IMMEDIATE` của lô ingest không khoá
    /// user.db nên ghi yêu thích/collection không phải chờ. Không migrate (cần `Store::open` đã chạy);
    /// chỉ gọi hàm thuần library (`rescan_root_with`, `ingest`…), hàm dùng bảng `u.*` sẽ lỗi `no such table`.
    pub fn open_library_only(dir: &Path) -> Result<Store, StoreError> {
        let conn = open_library_existing(&dir.join(LIBRARY_FILE))?;
        Ok(Store { conn, dir: dir.to_path_buf() })
    }

    /// Kết nối chỉ chạm user.db (không mở library.db): dành cho mọi ghi dữ liệu người dùng,
    /// để không tranh khoá với lô ingest. `main` là DB trong bộ nhớ; user.db attach thành `u`
    /// nên SQL của user.rs chạy nguyên. Không migrate: cần `Store::open` đã chạy trước.
    pub fn open_user_only(dir: &Path) -> Result<Store, StoreError> {
        let user_path = dir.join(USER_FILE);
        check_user_version(&user_path)?;
        let conn = Connection::open_in_memory()?;
        conn.execute("ATTACH DATABASE ?1 AS u", [user_path.to_string_lossy()])?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; PRAGMA u.synchronous=NORMAL;")?;
        Ok(Store { conn, dir: dir.to_path_buf() })
    }

    /// Kết nối chỉ đọc (library.db + user.db attach `u`), không migrate: dựng Index mà không
    /// giữ khoá của Store chính.
    pub fn open_reader(dir: &Path) -> Result<Store, StoreError> {
        let user_path = dir.join(USER_FILE);
        check_user_version(&user_path)?;
        let conn = Connection::open_with_flags(dir.join(LIBRARY_FILE), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.execute("ATTACH DATABASE ?1 AS u", [user_path.to_string_lossy()])?;
        conn.execute_batch("PRAGMA query_only=ON;")?;
        Ok(Store { conn, dir: dir.to_path_buf() })
    }

    /// Kết nối thô — cho chẩn đoán và test.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

fn open_library_existing(path: &Path) -> Result<Connection, StoreError> {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    c.busy_timeout(BUSY_TIMEOUT)?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
    Ok(c)
}

fn check_user_version(user_path: &Path) -> Result<(), StoreError> {
    let ro = Connection::open_with_flags(user_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let found = user_version(&ro)?;
    if found != USER_VERSION {
        return Err(StoreError::NewerSchema { db: "user", found, supported: USER_VERSION });
    }
    Ok(())
}

fn open_library(path: &Path, backup_dir: &Path) -> Result<Connection, StoreError> {
    let c = Connection::open(path)?;
    c.busy_timeout(BUSY_TIMEOUT)?;
    let tables: i64 = c.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
    if tables == 0 {
        // page_size phải đặt trước bảng đầu tiên và trước khi bật WAL.
        c.execute_batch("PRAGMA page_size=16384;")?;
    }
    c.execute_batch("PRAGMA journal_mode=WAL;")?;
    migrate(&c, "library", LIBRARY_MIGRATIONS, Some(backup_dir))?;
    Ok(c)
}

fn is_transient(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::DatabaseBusy
            | ErrorCode::DatabaseLocked
            | ErrorCode::DiskFull
            | ErrorCode::SystemIoFailure
            | ErrorCode::ReadOnly
            | ErrorCode::CannotOpen
            | ErrorCode::OutOfMemory
            | ErrorCode::OperationInterrupted
            | ErrorCode::PermissionDenied
    )
}

/// library.db là dữ liệu dẫn xuất: mới hơn app, migration lỗi hay file hỏng đều dựng lại.
fn library_is_broken(e: &StoreError) -> bool {
    match e {
        StoreError::NewerSchema { .. } => true,
        // lỗi tạm thời (bận/khoá, đầy đĩa, I/O, chỉ đọc, không mở được, hết bộ nhớ) KHÔNG phải DB hỏng:
        // trả lỗi và giữ nguyên file, nếu không người dùng thấy thư viện rỗng chỉ vì ổ đĩa đầy một lúc.
        StoreError::Migration { source: rusqlite::Error::SqliteFailure(f, _), .. } => !is_transient(f.code),
        StoreError::Migration { .. } => true,
        StoreError::Sqlite(rusqlite::Error::SqliteFailure(f, _)) => {
            matches!(f.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
        }
        _ => false,
    }
}

fn move_aside(lib_path: &Path) -> Result<(), StoreError> {
    let dir = lib_path.parent().unwrap_or(Path::new("."));
    let stamp = now_ms();
    for ext in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{}{ext}", lib_path.display()));
        if from.exists() {
            std::fs::rename(&from, dir.join(format!("library.broken-{stamp}.db{ext}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_migration_errors_keep_the_db() {
        for code in [
            ErrorCode::DiskFull,
            ErrorCode::SystemIoFailure,
            ErrorCode::ReadOnly,
            ErrorCode::CannotOpen,
            ErrorCode::OutOfMemory,
            ErrorCode::PermissionDenied,
        ] {
            let source = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { code, extended_code: 13 }, None);
            let e = StoreError::Migration { db: "library", from: 2, to: 3, source };
            assert!(!library_is_broken(&e), "{code:?} không được làm dựng lại library");
        }
        let notadb = StoreError::Migration {
            db: "library",
            from: 2,
            to: 3,
            source: rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { code: ErrorCode::NotADatabase, extended_code: 26 }, None),
        };
        assert!(library_is_broken(&notadb));
    }

    /// SQLITE_FULL thật: giới hạn số trang rồi chạy migration cần thêm trang.
    #[test]
    fn real_disk_full_during_migration_is_transient_and_rolls_back() {
        let d = tempfile::tempdir().unwrap();
        let c = Connection::open(d.path().join("library.db")).unwrap();
        crate::schema::create_library_v1_for_test(&c);
        let pages: i64 = c.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
        c.execute_batch(&format!("PRAGMA max_page_count = {pages};")).unwrap();
        let big = "CREATE TABLE zz(a); WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i < 200000) \
                   INSERT INTO zz SELECT hex(randomblob(64)) FROM n;";
        let err = crate::schema::migrate(&c, "library", &[crate::schema::LIBRARY_MIGRATIONS[0], big], None).unwrap_err();
        assert!(matches!(&err, StoreError::Migration { source: rusqlite::Error::SqliteFailure(f, _), .. } if f.code == ErrorCode::DiskFull), "{err:?}");
        assert!(!library_is_broken(&err));
        let v: u32 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 1, "bước lỗi rollback, version giữ nguyên");
    }

    #[test]
    fn busy_or_locked_migration_is_not_broken() {
        for code in [ErrorCode::DatabaseBusy, ErrorCode::DatabaseLocked] {
            let source = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { code, extended_code: 5 }, None);
            let e = StoreError::Migration { db: "library", from: 1, to: 2, source };
            assert!(!library_is_broken(&e), "{code:?} không được làm dựng lại library");
        }
        let corrupt = StoreError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: ErrorCode::DatabaseCorrupt, extended_code: 11 },
            None,
        ));
        assert!(library_is_broken(&corrupt));
    }
}
