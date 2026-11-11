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
    /// Bản thua đang bật trước khi ẩn (lớp ứng dụng tắt nó SAU khi sổ đã ghi).
    pub was_active: bool,
    /// Phạm vi activation của bản thua trước khi tắt (session | persistent), để hoàn tác bật lại đúng.
    pub loser_scope: Option<String>,
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
    /// pending = ghi sổ rồi nhưng chưa xong (đã ẩn); done = xong.
    pub status: String,
    pub loser_scope: Option<String>,
    /// Ta đã bật bản giữ giúp người dùng; hoàn tác tắt lại.
    pub keeper_activated: bool,
}

const COLS: &str = "id,batch_id,kind,group_key,keeper_hash,keeper_face,keeper_path,loser_hash,loser_face,loser_path,\
                    action,trash_path,reason,was_active,created_at,undone_at,\
                    (SELECT count(*) FROM u.dup_link_moves m WHERE m.resolution_id = dup_resolutions.id),\
                    status,loser_scope,keeper_activated";

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
        status: r.get(17)?,
        loser_scope: r.get(18)?,
        keeper_activated: r.get::<_, i64>(19)? != 0,
    })
}

fn journal(c: &Connection, id: i64, tbl: &str, op: &str, ref_id: i64, side: &str) -> Result<(), StoreError> {
    c.execute(
        "INSERT INTO u.dup_link_moves(resolution_id,tbl,op,ref_id,side) VALUES (?1,?2,?3,?4,?5)",
        params![id, tbl, op, ref_id, side],
    )?;
    Ok(())
}

/// Quyết định SỐNG khác (cùng bản giữ) cũng đã ghi sổ chính liên kết này? `created_at` = Some(t): chỉ xét quyết định tạo cùng
/// thời điểm t (cùng mẻ: dòng liên kết có dấu thời gian t là của chung cả mẻ).
fn other_live_journal(c: &Connection, id: i64, keeper: &FaceKey, tbl: &str, ref_id: i64, created_at: Option<i64>) -> Result<bool, StoreError> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM u.dup_link_moves m JOIN u.dup_resolutions r ON r.id = m.resolution_id
          WHERE m.tbl=?1 AND m.ref_id=?2 AND m.op='insert' AND r.id<>?3 AND r.undone_at IS NULL
            AND r.keeper_hash=?4 AND r.keeper_face=?5 AND (?6 IS NULL OR r.created_at=?6))",
        params![tbl, ref_id, id, &keeper.hash[..], keeper.face_index as i64, created_at],
        |r| r.get(0),
    )?)
}

/// Ghi sổ một liên kết đã chép. Khi INSERT OR IGNORE không thêm gì vì chính một quyết định cùng mẻ (cùng bản giữ, cùng thời
/// điểm) vừa thêm nó, vẫn ghi sổ cho quyết định này: dòng đó thuộc chung cả hai, chỉ xoá khi cả hai đã hoàn tác.
fn insert_link(c: &Connection, id: i64, keeper: &FaceKey, now: i64, tbl: &str, ref_id: i64, changed: usize) -> Result<(), StoreError> {
    if changed > 0 || other_live_journal(c, id, keeper, tbl, ref_id, Some(now))? {
        journal(c, id, tbl, "insert", ref_id, "")?;
    }
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

    let n = c.execute(
        "INSERT OR IGNORE INTO u.favorites(hash,face_index,created_at)
         SELECT ?1,?2,?5 FROM u.favorites WHERE hash=?3 AND face_index=?4",
        params![kh, kf, lh, lf, now],
    )?;
    insert_link(c, id, keeper, now, "favorites", 0, n)?;
    let mut ids = c.prepare("SELECT collection_id FROM u.collection_items WHERE hash=?1 AND face_index=?2")?;
    let cols = ids.query_map(params![lh, lf], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for col in cols {
        let n = c.execute(
            "INSERT OR IGNORE INTO u.collection_items(collection_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
            params![col, kh, kf, now],
        )?;
        insert_link(c, id, keeper, now, "collection_items", col, n)?;
    }
    let mut ids = c.prepare("SELECT tag_id FROM u.face_tags WHERE hash=?1 AND face_index=?2")?;
    let tags = ids.query_map(params![lh, lf], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for t in tags {
        let n = c.execute(
            "INSERT OR IGNORE INTO u.face_tags(tag_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
            params![t, kh, kf, now],
        )?;
        insert_link(c, id, keeper, now, "face_tags", t, n)?;
    }
    let mut ids = c.prepare("SELECT case_id, origin, note FROM u.case_faces WHERE hash=?1 AND face_index=?2")?;
    let cases = ids
        .query_map(params![lh, lf], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (case, origin, note) in cases {
        let n = c.execute(
            "INSERT OR IGNORE INTO u.case_faces(case_id,hash,face_index,origin,note,added_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![case, kh, kf, origin, note, now],
        )?;
        insert_link(c, id, keeper, now, "case_faces", case, n)?;
    }
    // Cặp: chỉ cặp (bản thua, bản giữ) mới tránh đổi trỏ — đổi sẽ tạo cặp tự trỏ (A,A). Cặp đã tự trỏ ở bản thua
    // (B,B) thì chuyển cả hai phía. Quyết định theo trạng thái cặp TRƯỚC khi sửa.
    let mut st = c.prepare(
        "SELECT pair_id, a_hash=?1 AND a_face=?2, b_hash=?1 AND b_face=?2, a_hash=?3 AND a_face=?4, b_hash=?3 AND b_face=?4
         FROM u.pairs WHERE (a_hash=?1 AND a_face=?2) OR (b_hash=?1 AND b_face=?2)",
    )?;
    let pairs = st
        .query_map(params![lh, lf, kh, kf], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, bool>(1)?, r.get::<_, bool>(2)?, r.get::<_, bool>(3)?, r.get::<_, bool>(4)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (p, a_loser, b_loser, a_keeper, b_keeper) in pairs {
        if (a_loser && b_keeper) || (b_loser && a_keeper) {
            continue;
        }
        for (on, side, hc, fc) in [(a_loser, "a", "a_hash", "a_face"), (b_loser, "b", "b_hash", "b_face")] {
            if on {
                c.execute(&format!("UPDATE u.pairs SET {hc}=?1, {fc}=?2 WHERE pair_id=?3"), params![kh, kf, p])?;
                journal(c, id, "pairs", "update", p, side)?;
            }
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
        // dòng chung với một quyết định sống khác (cùng bản giữ): chỉ quyết định cuối cùng hoàn tác mới được xoá
        if op == "insert" && other_live_journal(c, r.id, &r.keeper, &tbl, ref_id, None)? {
            continue;
        }
        match (tbl.as_str(), op.as_str()) {
            // chỉ xoá dòng do CHÍNH lần này tạo: dấu thời gian trùng `created_at` của quyết định. Người dùng xoá rồi
            // tạo lại dòng đó về sau sẽ có dấu thời gian khác → được giữ nguyên.
            ("favorites", "insert") => {
                c.execute("DELETE FROM u.favorites WHERE hash=?1 AND face_index=?2 AND created_at=?3", params![kh, kf, r.created_at])?;
            }
            ("collection_items", "insert") => {
                c.execute(
                    "DELETE FROM u.collection_items WHERE collection_id=?1 AND hash=?2 AND face_index=?3 AND added_at=?4",
                    params![ref_id, kh, kf, r.created_at],
                )?;
            }
            ("face_tags", "insert") => {
                c.execute(
                    "DELETE FROM u.face_tags WHERE tag_id=?1 AND hash=?2 AND face_index=?3 AND added_at=?4",
                    params![ref_id, kh, kf, r.created_at],
                )?;
            }
            ("case_faces", "insert") => {
                c.execute(
                    "DELETE FROM u.case_faces WHERE case_id=?1 AND hash=?2 AND face_index=?3 AND added_at=?4",
                    params![ref_id, kh, kf, r.created_at],
                )?;
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

fn begin_in(tx: &Connection, batch_id: &str, it: &NewResolution, now: i64) -> Result<i64, StoreError> {
    if it.keeper_path == it.loser_path {
        return Err(StoreError::Invalid("bản giữ và bản thua trùng đường dẫn".into()));
    }
    if !matches!(it.action.as_str(), "hide" | "trash") {
        return Err(StoreError::Invalid(format!("action không hợp lệ: {}", it.action)));
    }
    // bản giữ không được là bản đã bị ẩn bởi một quyết định sống
    let keeper_hidden: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM u.dup_resolutions WHERE loser_path=?1 AND undone_at IS NULL)",
        [&it.keeper_path],
        |r| r.get(0),
    )?;
    if keeper_hidden {
        return Err(StoreError::Invalid("bản giữ đang bị ẩn bởi một quyết định khác".into()));
    }
    tx.execute(
        "INSERT INTO u.dup_resolutions(batch_id,kind,group_key,keeper_hash,keeper_face,keeper_path,
            loser_hash,loser_face,loser_path,action,trash_path,reason,was_active,created_at,status,loser_scope)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'pending',?15)",
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
            now,
            it.loser_scope
        ],
    )?;
    let id = tx.last_insert_rowid();
    move_links(tx, id, &it.keeper, &it.loser, now)?;
    Ok(id)
}

/// Số liệu của một thành viên nhóm trùng cần cho quy tắc chọn bản giữ (không có trong `FaceRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub source_id: i64,
    pub num_glyphs: Option<u32>,
    /// Số face trong tệp (> 1 = TTC/OTC).
    pub num_faces: u32,
    /// Kích thước / mtime (ns) đã ghi lúc quét: dùng để biết tệp trên đĩa còn là tệp đã quét.
    pub size: u64,
    pub mtime_ns: i64,
}

impl Store {
    /// Thông tin của (face, đường dẫn); `None` nếu vị trí không còn trong thư viện.
    pub fn member_info(&self, key: &FaceKey, path: &str) -> Result<Option<MemberInfo>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT l.source_id, f.num_glyphs, fi.num_faces, l.size, l.mtime_ns
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
                        size: r.get::<_, i64>(3)?.max(0) as u64,
                        mtime_ns: r.get(4)?,
                    })
                },
            )
            .optional()?)
    }
}

impl Store {
    /// Ghi nhớ nhóm bị bỏ qua (gọi lại không lỗi). `kind` phải là same_file | versions | formats.
    pub fn ignore_group(&mut self, kind: &str, key: &str, now: i64) -> Result<(), StoreError> {
        if !matches!(kind, "same_file" | "versions" | "formats") {
            return Err(StoreError::Invalid(format!("loại nhóm không hợp lệ: {kind}")));
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO u.dup_ignored(kind,group_key,created_at) VALUES (?1,?2,?3)",
            params![kind, key, now],
        )?;
        Ok(())
    }

    /// Bỏ ghi nhớ; `true` nếu trước đó có.
    pub fn unignore_group(&mut self, kind: &str, key: &str) -> Result<bool, StoreError> {
        Ok(self.conn.execute("DELETE FROM u.dup_ignored WHERE kind=?1 AND group_key=?2", params![kind, key])? > 0)
    }

    /// (kind, key, created_at), mới nhất trước.
    pub fn ignored_groups(&self) -> Result<Vec<(String, String, i64)>, StoreError> {
        let mut st = self.conn.prepare("SELECT kind, group_key, created_at FROM u.dup_ignored ORDER BY created_at DESC, kind, group_key")?;
        Ok(st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Id nguồn theo thứ tự ưu tiên giữ bản trùng: nguồn đã sắp (theo `priority`), rồi nguồn chưa sắp theo thứ tự tạo.
    pub fn source_priority(&self) -> Result<Vec<i64>, StoreError> {
        let mut st = self.conn.prepare("SELECT source_id FROM u.sources ORDER BY priority IS NULL, priority, source_id")?;
        Ok(st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Đặt thứ tự: `ids` là phần đầu danh sách ưu tiên; nguồn không nêu giữ thứ tự cũ và xếp sau. Id lạ hoặc lặp → lỗi, không đổi gì.
    pub fn set_source_priority(&mut self, ids: &[i64]) -> Result<(), StoreError> {
        let current = self.source_priority()?;
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            if !current.contains(id) {
                return Err(StoreError::NotFound(format!("nguồn #{id}")));
            }
            if !seen.insert(*id) {
                return Err(StoreError::Invalid(format!("nguồn #{id} xuất hiện hai lần")));
            }
        }
        let full: Vec<i64> = ids.iter().copied().chain(current.iter().copied().filter(|i| !seen.contains(i))).collect();
        self.with_tx(|tx| {
            for (pos, id) in full.iter().enumerate() {
                tx.execute("UPDATE u.sources SET priority=?2 WHERE source_id=?1", params![id, pos as i64])?;
            }
            Ok(())
        })
    }
}

impl Store {
    /// Ghi một mẻ quyết định trong MỘT transaction (lỗi → không đổi gì) và chuyển liên kết sang bản giữ.
    /// Trả id theo thứ tự `items`. Một vị trí chỉ có thể bị ẩn bởi một quyết định đang sống.
    pub fn apply_resolutions(&mut self, batch_id: &str, items: &[NewResolution], now: i64) -> Result<Vec<i64>, StoreError> {
        self.with_tx(|tx| {
            let mut ids = Vec::with_capacity(items.len());
            for it in items {
                let id = begin_in(tx, batch_id, it, now)?;
                tx.execute("UPDATE u.dup_resolutions SET status='done' WHERE id=?1", [id])?;
                ids.push(id);
            }
            Ok(ids)
        })
    }

    /// Pha 1 của giải quyết: ghi sổ ở trạng thái `pending` và chuyển liên kết, MỘT transaction. Từ lúc này vị trí thua
    /// đã bị ẩn; mọi thay đổi ngoài sổ (tắt font, chuyển Thùng rác) làm SAU lệnh này, nên sổ luôn đủ để khôi phục.
    pub fn begin_resolution(&mut self, batch_id: &str, it: &NewResolution, now: i64) -> Result<i64, StoreError> {
        self.with_tx(|tx| begin_in(tx, batch_id, it, now))
    }

    /// Pha 2: đánh dấu xong; `trash_path` là nơi tệp nằm trong Thùng rác (chỉ khi action = trash).
    pub fn complete_resolution(&mut self, id: i64, trash_path: Option<&str>) -> Result<(), StoreError> {
        let n = self.conn.execute(
            "UPDATE u.dup_resolutions SET status='done', trash_path=COALESCE(?2, trash_path) WHERE id=?1 AND undone_at IS NULL",
            params![id, trash_path],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("quyết định #{id} đang sống")));
        }
        Ok(())
    }

    /// Ghi nhận ta đã bật bản giữ (để hoàn tác tắt lại).
    pub fn set_keeper_activated(&mut self, id: i64, on: bool) -> Result<(), StoreError> {
        self.conn.execute("UPDATE u.dup_resolutions SET keeper_activated=?2 WHERE id=?1", params![id, on as i64])?;
        Ok(())
    }

    /// Quyết định sống, MỚI HƠN `id`, mà bản thua chính là bản giữ của `id` (chuỗi A→B→C): hoàn tác `id` trước chúng là sai.
    pub fn newer_dependents(&self, id: i64) -> Result<Vec<i64>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT n.id FROM u.dup_resolutions n JOIN u.dup_resolutions r ON r.id = ?1
             WHERE n.undone_at IS NULL AND n.id > r.id AND n.loser_path = r.keeper_path ORDER BY n.id",
        )?;
        Ok(st.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Quyết định đang `pending` (còn làm dở): sau sự cố giữa chừng.
    pub fn pending_resolutions(&self) -> Result<Vec<ResolutionRow>, StoreError> {
        Ok(self.resolutions(true)?.into_iter().filter(|r| r.status == "pending").collect())
    }

    /// Đánh dấu một vị trí missing / có mặt NGAY (không chờ quét lại). Các worker chạy trên kết nối chỉ-library (nhúng, ngôn
    /// ngữ, quét) không thấy sổ `u.dup_resolutions` nên không biết vị trí đã ẩn; với tệp đã chuyển vào Thùng rác, cờ này
    /// là cách để chúng bỏ qua vị trí đó (và quét lại sau cũng đồng ý).
    pub fn set_location_missing(&mut self, path: &str, missing: bool) -> Result<bool, StoreError> {
        Ok(self.conn.execute("UPDATE main.file_locations SET missing=?2 WHERE path=?1 AND missing<>?2", params![path, missing as i64])? > 0)
    }

    /// `path` đang là bản thua của một quyết định sống (đã ẩn)?
    pub fn is_hidden_path(&self, path: &str) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM u.dup_resolutions WHERE loser_path=?1 AND undone_at IS NULL", [path], |_| Ok(()))
            .optional()?
            .is_some())
    }

    /// Hoàn tác một quyết định: gỡ liên kết đã chuyển, bỏ ẩn. File trong Thùng rác do lớp ứng dụng đưa về TRƯỚC khi gọi.
    /// Trả dòng đã hoàn tác (có `undone_at`). `Invalid` nếu đã hoàn tác rồi hoặc còn quyết định mới hơn phụ thuộc
    /// (hoàn tác theo thứ tự ngược: mới nhất trước); `NotFound` nếu không có id.
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
            let dep: i64 = tx.query_row(
                "SELECT count(*) FROM u.dup_resolutions n WHERE n.undone_at IS NULL AND n.id > ?1 AND n.loser_path = ?2",
                params![id, r.keeper_path],
                |x| x.get(0),
            )?;
            if dep > 0 {
                return Err(StoreError::Invalid(format!("quyết định #{id}: còn quyết định mới hơn dùng bản giữ làm bản thua")));
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
