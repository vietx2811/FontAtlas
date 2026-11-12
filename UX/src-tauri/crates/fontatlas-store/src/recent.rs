//! Font vừa xem (user.db): nguồn "Recent" của Desk. Cục bộ, tối đa `RECENT_MAX` dòng, xoá được.
use rusqlite::params;

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

pub const RECENT_MAX: usize = 50;

impl Store {
    /// Ghi một lần xem: xem lại face cũ thì đẩy lên đầu; cắt còn `RECENT_MAX` dòng mới nhất.
    pub fn record_view(&mut self, k: &FaceKey) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        // viewed_at đơn điệu tăng ngay cả khi hai lần ghi cùng mili-giây (thứ tự mới nhất trước luôn xác định)
        let now: i64 = tx.query_row(
            "SELECT max(?1, coalesce((SELECT max(viewed_at) + 1 FROM u.recent_faces), 0))",
            [now_ms()],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO u.recent_faces(hash,face_index,viewed_at) VALUES (?1,?2,?3)
             ON CONFLICT(hash,face_index) DO UPDATE SET viewed_at = excluded.viewed_at",
            params![&k.hash[..], k.face_index, now],
        )?;
        tx.execute(
            "DELETE FROM u.recent_faces WHERE (hash,face_index) NOT IN
               (SELECT hash,face_index FROM u.recent_faces ORDER BY viewed_at DESC LIMIT ?1)",
            [RECENT_MAX as i64],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Mới nhất trước, tối đa `limit` (≤ RECENT_MAX).
    pub fn recent_views(&self, limit: usize) -> Result<Vec<(FaceKey, i64)>, StoreError> {
        let mut st = self.conn.prepare("SELECT hash,face_index,viewed_at FROM u.recent_faces ORDER BY viewed_at DESC LIMIT ?1")?;
        let mut rows = st.query([limit.min(RECENT_MAX) as i64])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push((FaceKey { hash: hash_from_blob(r.get(0)?)?, face_index: r.get::<_, i64>(1)? as u32 }, r.get(2)?));
        }
        Ok(out)
    }

    /// Xoá toàn bộ lịch sử xem; trả số dòng đã xoá.
    pub fn clear_recent_views(&mut self) -> Result<usize, StoreError> {
        Ok(self.conn.execute("DELETE FROM u.recent_faces", [])?)
    }
}
