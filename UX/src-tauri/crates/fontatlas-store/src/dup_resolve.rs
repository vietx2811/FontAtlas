//! Sổ giải quyết trùng lặp (user.db): ẩn bản thua THEO VỊ TRÍ TỆP, chuyển liên kết sang bản giữ và hoàn tác chính xác.
//! File không bao giờ bị đụng ở đây: chuyển vào Thùng rác là việc của lớp ứng dụng, chỉ ghi lại `trash_path`.
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::Store;

/// Điều kiện SQL: vị trí `l` (bí danh của `file_locations`) đang bị ẩn bởi một quyết định chưa hoàn tác.
pub(crate) const HIDDEN_LOC: &str =
    "EXISTS (SELECT 1 FROM u.dup_resolutions dr WHERE dr.loser_path = l.path AND dr.undone_at IS NULL)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewResolution {
    /// same_file | versions | formats
    pub kind: String,
    pub group_key: String,
    pub keeper: FaceKey,
    pub keeper_path: String,
    pub loser: FaceKey,
    pub loser_path: String,
    /// hide | trash
    pub action: String,
    /// Chỉ với `trash`: nơi file nằm trong Thùng rác (để hoàn tác đưa về).
    pub trash_path: Option<String>,
    /// JSON `{code, args}` để UI dựng câu lý do.
    pub reason: String,
    /// Bản thua đang bật trước khi ẩn (lớp ứng dụng đã tắt nó).
    pub was_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionRow {
    pub id: i64,
    pub batch_id: String,
    pub kind: String,
    pub group_key: String,
    pub keeper: FaceKey,
    pub keeper_path: String,
    pub loser: FaceKey,
    pub loser_path: String,
    pub action: String,
    pub trash_path: Option<String>,
    pub reason: String,
    pub was_active: bool,
    pub created_at: i64,
    pub undone_at: Option<i64>,
    /// Số liên kết đã chuyển sang bản giữ (yêu thích, collection, tag, Case, cặp, slot).
    pub links_moved: u32,
}

const COLS: &str = "id,batch_id,kind,group_key,keeper_hash,keeper_face,keeper_path,loser_hash,loser_face,loser_path,\
                    action,trash_path,reason,was_active,created_at,undone_at,\
                    (SELECT count(*) FROM u.dup_link_moves m WHERE m.resolution_id = dup_resolutions.id)";

fn row_of(r: &Row) -> Result<ResolutionRow, StoreError> {
    Ok(ResolutionRow {
        id: r.get(0)?,
        batch_id: r.get(1)?,
        kind: r.get(2)?,
        group_key: r.get(3)?,
        keeper: FaceKey { hash: hash_from_blob(r.get(4)?)?, face_index: r.get::<_, i64>(5)? as u32 },
        keeper_path: r.get(6)?,
        loser: FaceKey { hash: hash_from_blob(r.get(7)?)?, face_index: r.get::<_, i64>(8)? as u32 },
        loser_path: r.get(9)?,
        action: r.get(10)?,
        trash_path: r.get(11)?,
        reason: r.get(12)?,
        was_active: r.get::<_, i64>(13)? != 0,
        created_at: r.get(14)?,
        undone_at: r.get(15)?,
        links_moved: r.get::<_, i64>(16)? as u32,
    })
}

fn journal(c: &Connection, id: i64, tbl: &str, op: &str, ref_id: i64, side: &str) -> Result<(), StoreError> {
    c.execute(
        "INSERT INTO u.dup_link_moves(resolution_id,tbl,op,ref_id,side) VALUES (?1,?2,?3,?4,?5)",
        params![id, tbl, op, ref_id, side],
    )?;
    Ok(())
}

/// Chép liên kết của bản thua sang bản giữ (INSERT OR IGNORE: chỉ dòng thực sự mới được ghi sổ) và đổi trỏ cặp / slot.
/// Cùng (hash, face) thì không có gì để chuyển (vd hai đường dẫn của một tệp).
fn move_links(c: &Connection, id: i64, keeper: &FaceKey, loser: &FaceKey, now: i64) -> Result<(), StoreError> {
    if keeper == loser {
        return Ok(());
    }
    let (kh, kf) = (&keeper.hash[..], keeper.face_index as i64);
    let (lh, lf) = (&loser.hash[..], loser.face_index as i64);

    if c.execute(
        "INSERT OR IGNORE INTO u.favorites(hash,face_index,created_at)
         SELECT ?1,?2,?5 FROM u.favorites WHERE hash=?3 AND face_index=?4",
        params![kh, kf, lh, lf, now],
    )? > 0
    {
        journal(c, id, "favorites", "insert", 0, "")?;
    }
    let mut ids = c.prepare("SELECT collection_id FROM u.collection_items WHERE hash=?1 AND face_index=?2")?;
    let cols = ids.query_map(params![lh, lf], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for col in cols {
        if c.execute(
            "INSERT OR IGNORE INTO u.collection_items(collection_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
            params![col, kh, kf, now],
        )? > 0
        {
            journal(c, id, "collection_items", "insert", col, "")?;
        }
    }
    let mut ids = c.prepare("SELECT tag_id FROM u.face_tags WHERE hash=?1 AND face_index=?2")?;
    let tags = ids.query_map(params![lh, lf], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for t in tags {
        if c.execute(
            "INSERT OR IGNORE INTO u.face_tags(tag_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
            params![t, kh, kf, now],
        )? > 0
        {
            journal(c, id, "face_tags", "insert", t, "")?;
        }
    }
    let mut ids = c.prepare("SELECT case_id, origin, note FROM u.case_faces WHERE hash=?1 AND face_index=?2")?;
    let cases = ids
        .query_map(params![lh, lf], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (case, origin, note) in cases {
        if c.execute(
            "INSERT OR IGNORE INTO u.case_faces(case_id,hash,face_index,origin,note,added_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![case, kh, kf, origin, note, now],
        )? > 0
        {
            journal(c, id, "case_faces", "insert", case, "")?;
        }
    }
    for (side, hc, fc) in [("a", "a_hash", "a_face"), ("b", "b_hash", "b_face")] {
        let mut st = c.prepare(&format!("SELECT pair_id FROM u.pairs WHERE {hc}=?1 AND {fc}=?2"))?;
        let ps = st.query_map(params![lh, lf], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for p in ps {
            c.execute(&format!("UPDATE u.pairs SET {hc}=?1, {fc}=?2 WHERE pair_id=?3"), params![kh, kf, p])?;
            journal(c, id, "pairs", "update", p, side)?;
        }
    }
    let mut st = c.prepare("SELECT combo_id, role FROM u.combo_slots WHERE hash=?1 AND face_index=?2")?;
    let slots = st
        .query_map(params![lh, lf], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (combo, role) in slots {
        c.execute("UPDATE u.combo_slots SET hash=?1, face_index=?2 WHERE combo_id=?3 AND role=?4", params![kh, kf, combo, role])?;
        journal(c, id, "combo_slots", "update", combo, &role)?;
    }
    Ok(())
}

/// Đảo từng dòng sổ. Chỉ gỡ / trả lại những gì chính lần giải quyết này đã đổi; dòng người dùng đã sửa / xoá từ đó được bỏ qua.
fn revert_links(c: &Connection, r: &ResolutionRow) -> Result<(), StoreError> {
    if r.keeper == r.loser {
        return Ok(());
    }
    let (kh, kf) = (&r.keeper.hash[..], r.keeper.face_index as i64);
    let (lh, lf) = (&r.loser.hash[..], r.loser.face_index as i64);
    let mut st = c.prepare("SELECT tbl, op, ref_id, side FROM u.dup_link_moves WHERE resolution_id=?1")?;
    let moves = st
        .query_map([r.id], |m| Ok((m.get::<_, String>(0)?, m.get::<_, String>(1)?, m.get::<_, i64>(2)?, m.get::<_, String>(3)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (tbl, op, ref_id, side) in moves {
        match (tbl.as_str(), op.as_str()) {
            ("favorites", "insert") => {
                c.execute("DELETE FROM u.favorites WHERE hash=?1 AND face_index=?2", params![kh, kf])?;
            }
            ("collection_items", "insert") => {
                c.execute(
                    "DELETE FROM u.collection_items WHERE collection_id=?1 AND hash=?2 AND face_index=?3",
                    params![ref_id, kh, kf],
                )?;
            }
            ("face_tags", "insert") => {
                c.execute("DELETE FROM u.face_tags WHERE tag_id=?1 AND hash=?2 AND face_index=?3", params![ref_id, kh, kf])?;
            }
            ("case_faces", "insert") => {
                c.execute("DELETE FROM u.case_faces WHERE case_id=?1 AND hash=?2 AND face_index=?3", params![ref_id, kh, kf])?;
            }
            ("pairs", "update") => {
                let (hc, fc) = if side == "a" { ("a_hash", "a_face") } else { ("b_hash", "b_face") };
                c.execute(
                    &format!("UPDATE u.pairs SET {hc}=?1, {fc}=?2 WHERE pair_id=?3 AND {hc}=?4 AND {fc}=?5"),
                    params![lh, lf, ref_id, kh, kf],
                )?;
            }
            ("combo_slots", "update") => {
                c.execute(
                    "UPDATE u.combo_slots SET hash=?1, face_index=?2 WHERE combo_id=?3 AND role=?4 AND hash=?5 AND face_index=?6",
                    params![lh, lf, ref_id, side, kh, kf],
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Số liệu của một thành viên nhóm trùng cần cho quy tắc chọn bản giữ (không có trong `FaceRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub source_id: i64,
    pub num_glyphs: Option<u32>,
    /// Số face trong tệp (> 1 = TTC/OTC).
    pub num_faces: u32,
}

impl Store {
    /// Thông tin của (face, đường dẫn); `None` nếu vị trí không còn trong thư viện.
    pub fn member_info(&self, key: &FaceKey, path: &str) -> Result<Option<MemberInfo>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT l.source_id, f.num_glyphs, fi.num_faces
                 FROM files fi
                 JOIN faces f ON f.file_id = fi.file_id AND f.face_index = ?2
                 JOIN file_locations l ON l.file_id = fi.file_id AND l.path = ?3
                 WHERE fi.hash = ?1",
                params![&key.hash[..], key.face_index as i64, path],
                |r| {
                    Ok(MemberInfo {
                        source_id: r.get(0)?,
                        num_glyphs: r.get::<_, Option<i64>>(1)?.map(|n| n as u32),
                        num_faces: r.get::<_, i64>(2)? as u32,
                    })
                },
            )
            .optional()?)
    }
}

impl Store {
    /// Ghi một mẻ quyết định trong MỘT transaction (lỗi → không đổi gì) và chuyển liên kết sang bản giữ.
    /// Trả id theo thứ tự `items`. Một vị trí chỉ có thể bị ẩn bởi một quyết định đang sống.
    pub fn apply_resolutions(&mut self, batch_id: &str, items: &[NewResolution], now: i64) -> Result<Vec<i64>, StoreError> {
        self.with_tx(|tx| {
            let mut ids = Vec::with_capacity(items.len());
            for it in items {
                if it.keeper_path == it.loser_path {
                    return Err(StoreError::Invalid("bản giữ và bản thua trùng đường dẫn".into()));
                }
                tx.execute(
                    "INSERT INTO u.dup_resolutions(batch_id,kind,group_key,keeper_hash,keeper_face,keeper_path,
                        loser_hash,loser_face,loser_path,action,trash_path,reason,was_active,created_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                    params![
                        batch_id,
                        it.kind,
                        it.group_key,
                        &it.keeper.hash[..],
                        it.keeper.face_index as i64,
                        it.keeper_path,
                        &it.loser.hash[..],
                        it.loser.face_index as i64,
                        it.loser_path,
                        it.action,
                        it.trash_path,
                        it.reason,
                        it.was_active as i64,
                        now
                    ],
                )?;
                let id = tx.last_insert_rowid();
                move_links(tx, id, &it.keeper, &it.loser, now)?;
                ids.push(id);
            }
            Ok(ids)
        })
    }

    /// Hoàn tác một quyết định: gỡ liên kết đã chuyển, bỏ ẩn. File trong Thùng rác do lớp ứng dụng đưa về TRƯỚC khi gọi.
    /// Trả dòng đã hoàn tác (có `undone_at`). `Invalid` nếu đã hoàn tác rồi, `NotFound` nếu không có id.
    pub fn undo_resolution(&mut self, id: i64, now: i64) -> Result<ResolutionRow, StoreError> {
        self.with_tx(|tx| {
            let mut r = tx
                .query_row(&format!("SELECT {COLS} FROM u.dup_resolutions WHERE id=?1"), [id], |r| Ok(row_of(r)))
                .optional()?
                .transpose()?
                .ok_or_else(|| StoreError::NotFound(format!("quyết định #{id}")))?;
            if r.undone_at.is_some() {
                return Err(StoreError::Invalid(format!("quyết định #{id} đã hoàn tác")));
            }
            revert_links(tx, &r)?;
            tx.execute("UPDATE u.dup_resolutions SET undone_at=?2 WHERE id=?1", params![id, now])?;
            r.undone_at = Some(now);
            Ok(r)
        })
    }

    /// Các quyết định, mới nhất trước. `live_only`: chỉ những cái chưa hoàn tác.
    pub fn resolutions(&self, live_only: bool) -> Result<Vec<ResolutionRow>, StoreError> {
        let sql = format!(
            "SELECT {COLS} FROM u.dup_resolutions {} ORDER BY id DESC",
            if live_only { "WHERE undone_at IS NULL" } else { "" }
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map([], |r| Ok(row_of(r)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().collect()
    }

    pub fn resolution(&self, id: i64) -> Result<ResolutionRow, StoreError> {
        self.conn
            .query_row(&format!("SELECT {COLS} FROM u.dup_resolutions WHERE id=?1"), [id], |r| Ok(row_of(r)))
            .optional()?
            .transpose()?
            .ok_or_else(|| StoreError::NotFound(format!("quyết định #{id}")))
    }
}
