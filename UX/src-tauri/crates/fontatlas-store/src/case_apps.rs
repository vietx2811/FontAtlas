//! B5 App Font Sets: Case gắn với app. `case_apps(collection_id, bundle_id)`; lý do kích hoạt `caseapp:<collection_id>`.
//! Đơn vị khoá app là `bundle_id` (NSWorkspace). Một Case có thể gắn nhiều app; font của Case bật khi BẤT KỲ app nào đang chạy.
use rusqlite::params;
use serde::Serialize;

use crate::activation::ActivationReason;
use crate::error::StoreError;
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseApp {
    pub bundle_id: String,
    pub app_name: String,
}

/// Một Case có app gắn (cho bộ đồng bộ): id và các bundle id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBinding {
    pub collection_id: i64,
    pub bundles: Vec<String>,
}

fn valid_bundle(b: &str) -> bool {
    (3..=255).contains(&b.len()) && b.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
}

impl Store {
    /// Các app gắn với một Case (theo thứ tự thêm).
    pub fn case_apps(&self, collection_id: i64) -> Result<Vec<CaseApp>, StoreError> {
        let mut st = self.conn.prepare("SELECT bundle_id, app_name FROM u.case_apps WHERE collection_id=?1 ORDER BY added_at, bundle_id")?;
        let rows = st.query_map([collection_id], |r| Ok(CaseApp { bundle_id: r.get(0)?, app_name: r.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Thay toàn bộ danh sách app của Case. Bundle id sai định dạng → `Invalid`; Case không tồn tại → `NotFound`.
    pub fn set_case_apps(&mut self, collection_id: i64, apps: &[CaseApp]) -> Result<(), StoreError> {
        if let Some(bad) = apps.iter().find(|a| !valid_bundle(&a.bundle_id)) {
            return Err(StoreError::Invalid(format!("bundle id không hợp lệ: {}", bad.bundle_id)));
        }
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM u.collections WHERE collection_id=?1)", [collection_id], |r| r.get(0))?;
        if !exists {
            return Err(StoreError::NotFound(format!("collection {collection_id}")));
        }
        tx.execute("DELETE FROM u.case_apps WHERE collection_id=?1", [collection_id])?;
        let now = now_ms();
        for (i, a) in apps.iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO u.case_apps(collection_id, bundle_id, app_name, added_at) VALUES (?1,?2,?3,?4)",
                params![collection_id, a.bundle_id, a.app_name, now + i as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Mọi Case có ít nhất một app gắn.
    pub fn case_bindings(&self) -> Result<Vec<CaseBinding>, StoreError> {
        let mut st = self.conn.prepare("SELECT collection_id, bundle_id FROM u.case_apps ORDER BY collection_id, added_at, bundle_id")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out: Vec<CaseBinding> = vec![];
        for (id, b) in rows {
            match out.last_mut() {
                Some(l) if l.collection_id == id => l.bundles.push(b),
                _ => out.push(CaseBinding { collection_id: id, bundles: vec![b] }),
            }
        }
        Ok(out)
    }

    /// Các Case đang mang lý do `caseapp:<id>` trên ít nhất một file (để gỡ khi hết app chạy hoặc hết gắn).
    pub fn case_app_reason_collections(&self) -> Result<Vec<i64>, StoreError> {
        let mut st = self.conn.prepare("SELECT DISTINCT reason FROM u.activation_reasons WHERE reason LIKE 'caseapp:%'")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .iter()
            .filter_map(|r| match ActivationReason::parse_db(r) {
                Some(ActivationReason::CaseApp(id)) => Some(id),
                _ => None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(b: &str) -> CaseApp {
        CaseApp { bundle_id: b.into(), app_name: b.into() }
    }

    #[test]
    fn bundle_id_format() {
        assert!(valid_bundle("com.adobe.Illustrator"));
        assert!(valid_bundle("com.apple.TextEdit"));
        assert!(!valid_bundle("a"));
        assert!(!valid_bundle("com adobe"));
        assert!(!valid_bundle("com/adobe"));
        assert!(!valid_bundle(""));
    }

    #[test]
    fn reason_roundtrip() {
        let r = ActivationReason::CaseApp(42);
        assert_eq!(r.as_db(), "caseapp:42");
        assert_eq!(ActivationReason::parse_db("caseapp:42"), Some(r));
        assert_eq!(ActivationReason::parse_db("caseapp:"), None);
        assert_eq!(ActivationReason::parse_db("caseapp:4x"), None);
        assert!(ActivationReason::CaseApp(1) > ActivationReason::Collection(9));
        let _ = app("x.y.z");
    }
}
