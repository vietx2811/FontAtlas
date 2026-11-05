//! Nguồn font: danh sách, thêm (kiểm lồng nhau), xoá, quét lại.
use std::path::Path;
use std::sync::Arc;

use fontatlas_store::{Source, SourceKind, StoreError};

use crate::dto::library::SourceDto;
use crate::error::CmdError;
use crate::state::AppState;

fn root_available(p: &str) -> bool {
    let p = Path::new(p);
    p.is_dir() && std::fs::read_dir(p).is_ok()
}

fn dto(state: &AppState, s: Source) -> Result<SourceDto, CmdError> {
    let face_count: i64 = state.with_store(|st| {
        Ok(st
            .conn()
            .query_row(
                "SELECT count(*) FROM faces f WHERE EXISTS (SELECT 1 FROM file_locations l \
                 WHERE l.file_id = f.file_id AND l.source_id = ?1 AND l.missing = 0)",
                [s.id],
                |r| r.get(0),
            )
            .map_err(StoreError::from)?)
    })?;
    Ok(SourceDto {
        id: s.id,
        kind: s.kind.as_str().into(),
        available: s.path.as_deref().is_none_or(root_available),
        name: s.name,
        path: s.path,
        enabled: s.enabled,
        face_count: face_count.max(0) as u32,
        last_scan: state.scans.last_scan(s.id),
        scanning: state.scans.is_scanning(s.id),
    })
}

pub fn list_sources(state: &AppState) -> Result<Vec<SourceDto>, CmdError> {
    let sources = state.with_user(|u| Ok(u.sources()?))?;
    sources.into_iter().map(|s| dto(state, s)).collect()
}

pub fn add_source(state: &Arc<AppState>, path: &str) -> Result<SourceDto, CmdError> {
    let p = Path::new(path);
    if !p.is_dir() {
        return Err(CmdError::invalid_arg(format!("không phải thư mục: {path}")));
    }
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string());
    let id = match state.with_user(|u| Ok(u.add_source(SourceKind::Directory, &name, Some(p))?)) {
        Ok(id) => id,
        Err(mut e) if e.code == "nested_source" => {
            // tra thêm đường dẫn của nguồn đang chiếm chỗ
            let existing = e.detail.as_ref().and_then(|d| d["existingId"].as_i64());
            if let Some(eid) = existing
                && let Ok(src) = state.with_user(|u| Ok(u.source(eid)?))
            {
                e.detail = Some(serde_json::json!({ "existingId": eid, "existingPath": src.path }));
            }
            return Err(e);
        }
        Err(e) => return Err(e),
    };
    state.scans.enqueue(state, &[id]);
    let src = state.with_user(|u| Ok(u.source(id)?))?;
    dto(state, src)
}

pub fn remove_source(state: &Arc<AppState>, id: i64) -> Result<(), CmdError> {
    state.with_store(|s| Ok(s.remove_source(id)?))
}

/// `None` = mọi nguồn thư mục.
pub fn rescan_sources(state: &Arc<AppState>, ids: Option<Vec<i64>>) -> Result<(), CmdError> {
    let ids = match ids {
        Some(v) => v,
        None => crate::services::scan::directory_source_ids(state)?,
    };
    state.scans.enqueue(state, &ids);
    Ok(())
}
