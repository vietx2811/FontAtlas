//! Case và Combo (user.db v6): nơi người dùng gom font, ghép vai và so sánh.
//! Case xoá mềm (`deleted_at`) 30 ngày; Combo là tập vai → font của một Case.
use rusqlite::{OptionalExtension, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

pub const MAX_CASE_FACES: usize = 500;
pub const MAX_COMBOS: usize = 50;
pub const TRASH_DAYS: i64 = 30;
/// Vai hợp lệ trong một Combo (khớp CHECK của `combo_slots.role`).
pub const ROLES: [&str; 7] = ["display", "heading", "subheading", "body", "caption", "accent", "quote"];
const ORIGINS: [&str; 4] = ["added", "pinned", "brief", "peek"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseRow {
    pub id: i64,
    pub name: String,
    pub brief: Option<String>,
    pub settings: String,
    pub view_state: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub opened_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseSummary {
    pub id: i64,
    pub name: String,
    pub face_count: u32,
    pub combo_count: u32,
    pub updated_at: i64,
    pub opened_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFaceRow {
    pub key: FaceKey,
    pub origin: String,
    pub note: Option<String>,
    pub added_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRow {
    pub role: String,
    pub ord: i64,
    pub key: FaceKey,
    pub note: Option<String>,
    pub style: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboRow {
    pub id: i64,
    pub case_id: i64,
    pub idx: i64,
    pub name: String,
    pub is_main: bool,
    pub note: Option<String>,
    pub slots: Vec<SlotRow>,
}

/// `Option<Option<_>>`: `None` = giữ nguyên, `Some(None)` = xoá.
/// Kết quả `peek_open`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeekOutcome {
    pub case_id: i64,
    pub combo_id: i64,
    pub created: bool,
    /// Font đang giữ vai neo (khác neo) và không bị ghi đè vì `replace = false`.
    pub occupied: Option<FaceKey>,
}

#[derive(Debug, Clone, Default)]
pub struct CasePatch {
    pub name: Option<String>,
    pub brief: Option<Option<String>>,
    pub settings: Option<String>,
    pub view_state: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ComboPatch {
    pub name: Option<String>,
    pub is_main: Option<bool>,
    pub note: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct NewCombo {
    pub name: Option<String>,
    pub slots: Vec<(String, FaceKey)>,
}

/// Khối nguyên tử lồng được (SAVEPOINT): lỗi hoặc thả mà chưa `commit` thì hoàn tác. Dùng thay cho transaction
/// để các thao tác gộp lại (`peek_open`) vẫn là MỘT đơn vị dù mỗi bước đã tự bọc.
pub(crate) struct Atomic<'a> {
    conn: &'a rusqlite::Connection,
    done: bool,
}

impl<'a> Atomic<'a> {
    pub(crate) fn begin(conn: &'a rusqlite::Connection) -> rusqlite::Result<Atomic<'a>> {
        conn.execute_batch("SAVEPOINT atomic_block")?;
        Ok(Atomic { conn, done: false })
    }
    pub(crate) fn commit(mut self) -> rusqlite::Result<()> {
        self.done = true;
        self.conn.execute_batch("RELEASE atomic_block")
    }
}

impl std::ops::Deref for Atomic<'_> {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &rusqlite::Connection {
        self.conn
    }
}

impl Drop for Atomic<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.conn.execute_batch("ROLLBACK TO atomic_block; RELEASE atomic_block");
        }
    }
}

fn check_role(role: &str) -> Result<(), StoreError> {
    if ROLES.contains(&role) { Ok(()) } else { Err(StoreError::Invalid(format!("vai không hợp lệ: {role}"))) }
}

fn check_json(label: &str, s: &str) -> Result<(), StoreError> {
    serde_json::from_str::<serde_json::Value>(s)
        .map(|_| ())
        .map_err(|_| StoreError::Invalid(format!("{label} không phải JSON")))
}

fn clean_name(name: &str, what: &str) -> Result<String, StoreError> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 120 {
        return Err(StoreError::Invalid(format!("tên {what} rỗng hoặc quá dài (≤ 120)")));
    }
    Ok(n.to_string())
}

fn case_row(r: &rusqlite::Row) -> rusqlite::Result<CaseRow> {
    Ok(CaseRow {
        id: r.get(0)?,
        name: r.get(1)?,
        brief: r.get(2)?,
        settings: r.get(3)?,
        view_state: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        opened_at: r.get(7)?,
        deleted_at: r.get(8)?,
    })
}

const CASE_COLS: &str = "id,name,brief,settings,view_state,created_at,updated_at,opened_at,deleted_at";

impl Store {
    // ---- Case ----

    pub fn create_case(&mut self, name: &str, brief: Option<&str>, settings: Option<&str>) -> Result<i64, StoreError> {
        let name = clean_name(name, "Case")?;
        if let Some(b) = brief {
            check_json("brief", b)?;
        }
        let settings = settings.unwrap_or("{}");
        check_json("settings", settings)?;
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO u.cases(name,brief,settings,created_at,updated_at,opened_at) VALUES (?1,?2,?3,?4,?4,?4)",
            params![name, brief, settings, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Case chưa bị xoá. `touch` = đánh dấu vừa mở (`opened_at`).
    pub fn case(&self, id: i64) -> Result<CaseRow, StoreError> {
        self.case_any(id).and_then(|c| match c.deleted_at {
            None => Ok(c),
            Some(_) => Err(StoreError::NotFound(format!("Case #{id} đã xoá"))),
        })
    }

    pub(crate) fn case_any(&self, id: i64) -> Result<CaseRow, StoreError> {
        self.conn
            .query_row(&format!("SELECT {CASE_COLS} FROM u.cases WHERE id = ?1"), [id], case_row)
            .optional()?
            .ok_or_else(|| StoreError::NotFound(format!("Case #{id}")))
    }

    pub fn touch_case(&mut self, id: i64) -> Result<(), StoreError> {
        self.case(id)?;
        self.bump_opened(id)?;
        Ok(())
    }

    /// `opened_at` luôn lớn hơn mọi Case khác (kể cả khi hai lần mở rơi vào cùng một mili-giây).
    fn bump_opened(&self, id: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE u.cases SET opened_at = max(?2, (SELECT coalesce(max(opened_at), 0) + 1 FROM u.cases WHERE id <> ?1)) WHERE id = ?1",
            params![id, now_ms()],
        )?;
        Ok(())
    }

    /// Mới mở gần nhất, chưa xoá.
    pub fn latest_case(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT id FROM u.cases WHERE deleted_at IS NULL ORDER BY opened_at DESC, id DESC LIMIT 1", [], |r| r.get(0))
            .optional()?)
    }

    pub fn cases(&self, include_deleted: bool) -> Result<Vec<CaseSummary>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT c.id, c.name, (SELECT count(*) FROM u.case_faces f WHERE f.case_id = c.id),
                    (SELECT count(*) FROM u.combos b WHERE b.case_id = c.id), c.updated_at, c.opened_at, c.deleted_at
             FROM u.cases c WHERE (?1 OR c.deleted_at IS NULL) ORDER BY c.opened_at DESC, c.id DESC",
        )?;
        let rows = st.query_map([include_deleted], |r| {
            Ok(CaseSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                face_count: r.get::<_, i64>(2)? as u32,
                combo_count: r.get::<_, i64>(3)? as u32,
                updated_at: r.get(4)?,
                opened_at: r.get(5)?,
                deleted_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_case(&mut self, id: i64, p: &CasePatch) -> Result<CaseRow, StoreError> {
        let cur = self.case(id)?;
        let name = match &p.name {
            Some(n) => clean_name(n, "Case")?,
            None => cur.name,
        };
        let brief = match &p.brief {
            Some(Some(b)) => {
                check_json("brief", b)?;
                Some(b.clone())
            }
            Some(None) => None,
            None => cur.brief,
        };
        let settings = p.settings.clone().unwrap_or(cur.settings);
        let view = p.view_state.clone().unwrap_or(cur.view_state);
        check_json("settings", &settings)?;
        check_json("viewState", &view)?;
        self.conn.execute(
            "UPDATE u.cases SET name=?2, brief=?3, settings=?4, view_state=?5, updated_at=?6 WHERE id=?1",
            params![id, name, brief, settings, view, now_ms()],
        )?;
        self.case(id)
    }

    /// Xoá mềm. Idempotent với Case đã xoá (giữ mốc cũ).
    pub fn delete_case(&mut self, id: i64) -> Result<(), StoreError> {
        self.case_any(id)?;
        self.conn.execute("UPDATE u.cases SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL", params![id, now_ms()])?;
        Ok(())
    }

    pub fn restore_case(&mut self, id: i64) -> Result<(), StoreError> {
        self.case_any(id)?;
        self.conn.execute("UPDATE u.cases SET deleted_at = NULL WHERE id = ?1", [id])?;
        self.bump_opened(id)?;
        Ok(())
    }

    /// Xoá hẳn Case đã nằm trong thùng rác quá `TRASH_DAYS` ngày (tính theo `now`).
    /// Bỏ qua (trả 0) nếu đồng hồ có vẻ đã lùi: `now` nhỏ hơn mốc thời gian mới nhất đã ghi trong bảng.
    pub fn purge_cases(&mut self, now: i64) -> Result<usize, StoreError> {
        let latest: Option<i64> = self.conn.query_row(
            "SELECT max(max(created_at), max(updated_at), max(opened_at), coalesce(max(deleted_at), 0)) FROM u.cases",
            [],
            |r| r.get(0),
        )?;
        if latest.is_some_and(|l| now < l) {
            return Ok(0);
        }
        let cutoff = now - TRASH_DAYS * 24 * 3600 * 1000;
        Ok(self.conn.execute("DELETE FROM u.cases WHERE deleted_at IS NOT NULL AND deleted_at < ?1", [cutoff])?)
    }

    // ---- Font trong Case ----

    pub fn case_faces(&self, case_id: i64) -> Result<Vec<CaseFaceRow>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT hash, face_index, origin, note, added_at FROM u.case_faces WHERE case_id = ?1 ORDER BY added_at, hash, face_index")?;
        let rows = st.query_map([case_id], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, i64>(4)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (h, face_index, origin, note, added_at) = r?;
            out.push(CaseFaceRow { key: FaceKey { hash: hash_from_blob(h)?, face_index }, origin, note, added_at });
        }
        Ok(out)
    }

    /// Thêm font vào Case. Trả (số mới thêm, số đã có sẵn). Vượt `MAX_CASE_FACES` → `Invalid`, không thêm gì.
    pub fn add_to_case(&mut self, case_id: i64, keys: &[FaceKey], origin: &str) -> Result<(usize, usize), StoreError> {
        if !ORIGINS.contains(&origin) {
            return Err(StoreError::Invalid(format!("origin không hợp lệ: {origin}")));
        }
        self.case(case_id)?;
        let tx = Atomic::begin(&self.conn)?;
        let now = now_ms();
        let (mut added, mut already) = (0, 0);
        let mut count: i64 = tx.query_row("SELECT count(*) FROM u.case_faces WHERE case_id = ?1", [case_id], |r| r.get(0))?;
        for k in keys {
            let n = tx.execute(
                "INSERT OR IGNORE INTO u.case_faces(case_id,hash,face_index,origin,added_at) VALUES (?1,?2,?3,?4,?5)",
                params![case_id, &k.hash[..], k.face_index, origin, now],
            )?;
            if n == 1 {
                added += 1;
                count += 1;
                if count as usize > MAX_CASE_FACES {
                    return Err(StoreError::Invalid(format!("Case tối đa {MAX_CASE_FACES} font")));
                }
            } else {
                already += 1;
            }
        }
        tx.execute("UPDATE u.cases SET updated_at = ?2 WHERE id = ?1", params![case_id, now])?;
        tx.commit()?;
        Ok((added, already))
    }

    /// Bỏ font khỏi Case (kèm mọi vai đang dùng font đó trong Combo của Case). Trả số font đã bỏ.
    pub fn remove_from_case(&mut self, case_id: i64, keys: &[FaceKey]) -> Result<usize, StoreError> {
        self.case(case_id)?;
        let tx = Atomic::begin(&self.conn)?;
        let mut n = 0;
        for k in keys {
            tx.execute(
                "DELETE FROM u.combo_slots WHERE hash = ?2 AND face_index = ?3
                   AND combo_id IN (SELECT id FROM u.combos WHERE case_id = ?1)",
                params![case_id, &k.hash[..], k.face_index],
            )?;
            n += tx.execute(
                "DELETE FROM u.case_faces WHERE case_id=?1 AND hash=?2 AND face_index=?3",
                params![case_id, &k.hash[..], k.face_index],
            )?;
        }
        tx.execute("UPDATE u.cases SET updated_at = ?2 WHERE id = ?1", params![case_id, now_ms()])?;
        tx.commit()?;
        Ok(n)
    }

    /// "Pair from peek" trong MỘT khối nguyên tử: lấy/tạo Case, thêm font neo (và ứng viên) với origin `peek`,
    /// đảm bảo có Combo chính, đặt neo vào `anchor_role`. Không ghi đè ngầm:
    /// - `settings` chỉ được ghi khi Case được TẠO ở đây; Case có sẵn giữ nguyên cài đặt của nó;
    /// - vai đã có font KHÁC thì không ghi (`occupied`), trừ khi `replace`.
    ///
    /// Idempotent. Lỗi ở bất kỳ bước nào thì không để lại gì (kể cả Case vừa tạo, cài đặt cũ giữ nguyên).
    pub fn peek_open(
        &mut self,
        case_id: Option<i64>,
        settings: &str,
        anchor: &FaceKey,
        candidate: Option<&FaceKey>,
        anchor_role: &str,
        replace: bool,
    ) -> Result<PeekOutcome, StoreError> {
        check_json("settings", settings)?;
        check_role(anchor_role)?;
        self.conn.execute_batch("SAVEPOINT peek_open")?;
        let run = |s: &mut Store| -> Result<PeekOutcome, StoreError> {
            let (case_id, created) = match case_id {
                Some(id) => (id, false),
                None => match s.latest_case()? {
                    Some(id) => (id, false),
                    None => (s.create_case("Case 1", None, Some(settings))?, true),
                },
            };
            s.case(case_id)?;
            s.add_to_case(case_id, std::slice::from_ref(anchor), "peek")?;
            if let Some(c) = candidate {
                s.add_to_case(case_id, std::slice::from_ref(c), "peek")?;
            }
            let combo_id = match s.combos(case_id)?.into_iter().find(|c| c.is_main).map(|c| c.id) {
                Some(id) => id,
                None => s.create_combo(case_id, None, None)?,
            };
            let held = s.combo(combo_id)?.slots.into_iter().find(|x| x.role == anchor_role).map(|x| x.key);
            let occupied = match held {
                Some(k) if k != *anchor && !replace => Some(k),
                _ => {
                    s.set_combo_slot(combo_id, anchor_role, Some(anchor), None, None)?;
                    None
                }
            };
            s.touch_case(case_id)?;
            Ok(PeekOutcome { case_id, combo_id, created, occupied })
        };
        match run(self) {
            Ok(v) => {
                self.conn.execute_batch("RELEASE peek_open")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK TO peek_open; RELEASE peek_open");
                Err(e)
            }
        }
    }

    /// `add_to_case(None)` nguyên tử: Case mở gần nhất (tạo mới nếu chưa có) + thêm font + đánh dấu vừa mở, một khối.
    /// Trả (case_id, mới thêm, đã có, vừa tạo Case).
    pub fn add_to_latest_case(&mut self, keys: &[FaceKey], origin: &str) -> Result<(i64, usize, usize, bool), StoreError> {
        self.conn.execute_batch("SAVEPOINT add_latest")?;
        let run = |s: &mut Store| -> Result<(i64, usize, usize, bool), StoreError> {
            let (id, created) = match s.latest_case()? {
                Some(id) => (id, false),
                None => (s.create_case("Case 1", None, None)?, true),
            };
            let (added, already) = s.add_to_case(id, keys, origin)?;
            s.touch_case(id)?;
            Ok((id, added, already, created))
        };
        match run(self) {
            Ok(v) => {
                self.conn.execute_batch("RELEASE add_latest")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK TO add_latest; RELEASE add_latest");
                Err(e)
            }
        }
    }

    // ---- Combo ----

    fn slots_of(&self, combo_id: i64) -> Result<Vec<SlotRow>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT role, ord, hash, face_index, note, style FROM u.combo_slots WHERE combo_id = ?1 ORDER BY ord, role")?;
        let rows = st.query_map([combo_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Vec<u8>>(2)?, r.get::<_, u32>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, String>(5)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (role, ord, h, face_index, note, style) = r?;
            out.push(SlotRow { role, ord, key: FaceKey { hash: hash_from_blob(h)?, face_index }, note, style });
        }
        Ok(out)
    }

    pub fn combos(&self, case_id: i64) -> Result<Vec<ComboRow>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT id, case_id, idx, name, is_main, note FROM u.combos WHERE case_id = ?1 ORDER BY idx")?;
        let rows = st.query_map([case_id], |r| {
            Ok(ComboRow { id: r.get(0)?, case_id: r.get(1)?, idx: r.get(2)?, name: r.get(3)?, is_main: r.get::<_, i64>(4)? != 0, note: r.get(5)?, slots: vec![] })
        })?;
        let mut out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        for c in &mut out {
            c.slots = self.slots_of(c.id)?;
        }
        Ok(out)
    }

    pub fn combo(&self, id: i64) -> Result<ComboRow, StoreError> {
        let case_id: i64 = self
            .conn
            .query_row("SELECT case_id FROM u.combos WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .ok_or_else(|| StoreError::NotFound(format!("Combo #{id}")))?;
        self.combos(case_id)?.into_iter().find(|c| c.id == id).ok_or_else(|| StoreError::NotFound(format!("Combo #{id}")))
    }

    /// Tạo Combo ở cuối; Combo đầu tiên của Case là chính. `from` = nhân bản vai/font của Combo khác (cùng Case).
    pub fn create_combo(&mut self, case_id: i64, name: Option<&str>, from: Option<i64>) -> Result<i64, StoreError> {
        let tx = Atomic::begin(&self.conn)?;
        let id = self.create_combo_tx(&tx, case_id, name, from, &[])?;
        tx.commit()?;
        Ok(id)
    }

    fn create_combo_tx(
        &self,
        tx: &Atomic,
        case_id: i64,
        name: Option<&str>,
        from: Option<i64>,
        slots: &[(String, FaceKey)],
    ) -> Result<i64, StoreError> {
        self.case(case_id)?;
        let src = match from {
            Some(f) => {
                let c = self.live_combo(f)?;
                if c.case_id != case_id {
                    return Err(StoreError::Invalid("Combo nguồn thuộc Case khác".into()));
                }
                Some(c)
            }
            None => None,
        };
        let (n, max): (i64, Option<i64>) =
            tx.query_row("SELECT count(*), max(idx) FROM u.combos WHERE case_id = ?1", [case_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if n as usize >= MAX_COMBOS {
            return Err(StoreError::Invalid(format!("Case tối đa {MAX_COMBOS} Combo")));
        }
        let idx = max.map_or(1, |m| m + 1);
        let name = match name {
            Some(n) => clean_name(n, "Combo")?,
            None => format!("Combo {idx}"),
        };
        let now = now_ms();
        tx.execute(
            "INSERT INTO u.combos(case_id,idx,name,is_main,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?5)",
            params![case_id, idx, name, (n == 0) as i64, now],
        )?;
        let id = tx.last_insert_rowid();
        let put = |ord: i64, role: &str, k: &FaceKey, note: Option<&str>, style: &str| -> Result<(), StoreError> {
            check_role(role)?;
            tx.execute(
                "INSERT OR REPLACE INTO u.combo_slots(combo_id,role,ord,hash,face_index,note,style) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![id, role, ord, &k.hash[..], k.face_index, note, style],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO u.case_faces(case_id,hash,face_index,origin,added_at) VALUES (?1,?2,?3,'added',?4)",
                params![case_id, &k.hash[..], k.face_index, now],
            )?;
            Ok(())
        };
        if let Some(s) = &src {
            for sl in &s.slots {
                put(sl.ord, &sl.role, &sl.key, sl.note.as_deref(), &sl.style)?;
            }
        }
        let mut seen_roles: Vec<&str> = Vec::new();
        for (role, k) in slots {
            if seen_roles.contains(&role.as_str()) {
                return Err(StoreError::Invalid(format!("vai trùng trong một Combo: {role}")));
            }
            seen_roles.push(role);
            let ord = ROLES.iter().position(|r| r == role).unwrap_or(0) as i64;
            put(ord, role, k, None, "{}")?;
        }
        let total: i64 = tx.query_row("SELECT count(*) FROM u.case_faces WHERE case_id = ?1", [case_id], |r| r.get(0))?;
        if total as usize > MAX_CASE_FACES {
            return Err(StoreError::Invalid(format!("Case tối đa {MAX_CASE_FACES} font")));
        }
        tx.execute("UPDATE u.cases SET updated_at = ?2 WHERE id = ?1", params![case_id, now])?;
        Ok(id)
    }

    /// Nhiều Combo một transaction: lỗi ở Combo nào thì không Combo nào được tạo.
    pub fn create_combos(&mut self, case_id: i64, combos: &[NewCombo]) -> Result<Vec<i64>, StoreError> {
        self.case(case_id)?;
        let tx = Atomic::begin(&self.conn)?;
        let mut ids = Vec::new();
        for c in combos {
            ids.push(self.create_combo_tx(&tx, case_id, c.name.as_deref(), None, &c.slots)?);
        }
        tx.commit()?;
        Ok(ids)
    }

    /// Combo của Case chưa xoá (Case trong thùng rác không nhận ghi).
    fn live_combo(&self, id: i64) -> Result<ComboRow, StoreError> {
        let c = self.combo(id)?;
        self.case(c.case_id)?;
        Ok(c)
    }

    pub fn update_combo(&mut self, id: i64, p: &ComboPatch) -> Result<ComboRow, StoreError> {
        let cur = self.live_combo(id)?;
        let name = match &p.name {
            Some(n) => clean_name(n, "Combo")?,
            None => cur.name.clone(),
        };
        let note = match &p.note {
            Some(n) => n.clone(),
            None => cur.note.clone(),
        };
        let tx = Atomic::begin(&self.conn)?;
        if p.is_main == Some(true) {
            tx.execute("UPDATE u.combos SET is_main = 0 WHERE case_id = ?1", [cur.case_id])?;
        }
        let main = p.is_main.unwrap_or(cur.is_main);
        tx.execute("UPDATE u.combos SET name=?2, note=?3, is_main=?4, updated_at=?5 WHERE id=?1", params![id, name, note, main as i64, now_ms()])?;
        // luôn còn đúng một Combo chính nếu Case có Combo
        let mains: i64 = tx.query_row("SELECT count(*) FROM u.combos WHERE case_id=?1 AND is_main=1", [cur.case_id], |r| r.get(0))?;
        if mains == 0 {
            tx.execute("UPDATE u.combos SET is_main = 1 WHERE id = (SELECT id FROM u.combos WHERE case_id=?1 ORDER BY idx LIMIT 1)", [cur.case_id])?;
        }
        tx.commit()?;
        self.combo(id)
    }

    pub fn delete_combo(&mut self, id: i64) -> Result<(), StoreError> {
        let cur = self.live_combo(id)?;
        let tx = Atomic::begin(&self.conn)?;
        tx.execute("DELETE FROM u.combos WHERE id = ?1", [id])?;
        // đánh số lại liên tục 1..n và giữ một Combo chính
        let ids: Vec<i64> = {
            let mut st = tx.prepare("SELECT id FROM u.combos WHERE case_id = ?1 ORDER BY idx")?;
            st.query_map([cur.case_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        renumber(&tx, &ids)?;
        if cur.is_main && let Some(first) = ids.first() {
            tx.execute("UPDATE u.combos SET is_main = 1 WHERE id = ?1", [first])?;
        }
        tx.execute("UPDATE u.cases SET updated_at = ?2 WHERE id = ?1", params![cur.case_id, now_ms()])?;
        tx.commit()?;
        Ok(())
    }

    /// `ids` phải đúng là tập Combo của Case (mọi Combo, mỗi cái một lần).
    pub fn reorder_combos(&mut self, case_id: i64, ids: &[i64]) -> Result<(), StoreError> {
        self.case(case_id)?;
        let mut have: Vec<i64> = self.combos(case_id)?.iter().map(|c| c.id).collect();
        let mut want = ids.to_vec();
        have.sort();
        want.sort();
        if have != want {
            return Err(StoreError::Invalid("danh sách Combo không khớp Case".into()));
        }
        let tx = Atomic::begin(&self.conn)?;
        renumber(&tx, ids)?;
        tx.commit()?;
        Ok(())
    }

    /// Đặt (`Some`) hoặc bỏ (`None`) font ở một vai. Font tự được thêm vào Case.
    pub fn set_combo_slot(&mut self, combo_id: i64, role: &str, face: Option<&FaceKey>, note: Option<&str>, style: Option<&str>) -> Result<ComboRow, StoreError> {
        check_role(role)?;
        if let Some(s) = style {
            check_json("style", s)?;
        }
        let combo = self.live_combo(combo_id)?;
        let tx = Atomic::begin(&self.conn)?;
        let now = now_ms();
        match face {
            Some(k) => {
                let ord = ROLES.iter().position(|r| *r == role).unwrap_or(0) as i64;
                let prev = combo.slots.iter().find(|s| s.role == role);
                tx.execute(
                    "INSERT INTO u.combo_slots(combo_id,role,ord,hash,face_index,note,style) VALUES (?1,?2,?3,?4,?5,?6,?7)
                     ON CONFLICT(combo_id,role) DO UPDATE SET hash=excluded.hash, face_index=excluded.face_index,
                       note=excluded.note, style=excluded.style",
                    params![
                        combo_id, role, ord, &k.hash[..], k.face_index,
                        note.map(str::to_string).or_else(|| prev.and_then(|p| p.note.clone())),
                        style.map(str::to_string).or_else(|| prev.map(|p| p.style.clone())).unwrap_or_else(|| "{}".into())
                    ],
                )?;
                tx.execute(
                    "INSERT OR IGNORE INTO u.case_faces(case_id,hash,face_index,origin,added_at) VALUES (?1,?2,?3,'added',?4)",
                    params![combo.case_id, &k.hash[..], k.face_index, now],
                )?;
                let total: i64 = tx.query_row("SELECT count(*) FROM u.case_faces WHERE case_id = ?1", [combo.case_id], |r| r.get(0))?;
                if total as usize > MAX_CASE_FACES {
                    return Err(StoreError::Invalid(format!("Case tối đa {MAX_CASE_FACES} font")));
                }
            }
            None => {
                tx.execute("DELETE FROM u.combo_slots WHERE combo_id=?1 AND role=?2", params![combo_id, role])?;
            }
        }
        tx.execute("UPDATE u.combos SET updated_at = ?2 WHERE id = ?1", params![combo_id, now])?;
        tx.execute("UPDATE u.cases SET updated_at = ?2 WHERE id = ?1", params![combo.case_id, now])?;
        tx.commit()?;
        self.combo(combo_id)
    }
}

/// Đánh số lại `idx` = 1..n theo thứ tự `ids` (đi qua giá trị âm để không vi phạm UNIQUE).
fn renumber(tx: &Atomic, ids: &[i64]) -> Result<(), StoreError> {
    for id in ids {
        tx.execute("UPDATE u.combos SET idx = -id WHERE id = ?1", [id])?;
    }
    for (i, id) in ids.iter().enumerate() {
        tx.execute("UPDATE u.combos SET idx = ?2 WHERE id = ?1", params![id, i as i64 + 1])?;
    }
    Ok(())
}
