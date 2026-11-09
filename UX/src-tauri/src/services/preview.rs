//! Scheme `fontatlas-preview://` và `fontatlas-font://`, cùng lệnh preview. Handler thuần nằm ở fontatlas-preview.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fontatlas_preview::{
    Budgets, CorsPolicy, FaceLocator, PX_BUCKETS, PreviewCache, PreviewHttp, PreviewRequest, PreviewService, RENDERER_VERSION,
    handle_font_uri, handle_preview_uri,
};
use fontatlas_store::{FaceKey, Store};

use crate::dto::library::FaceKeyDto;
use crate::dto::preview::{PreviewCacheInfoDto, PreviewInfoDto, PreviewItemDto};
use crate::error::CmdError;
use crate::state::AppState;

/// Tra đường dẫn file theo (hash, face) bằng kết nối đọc riêng của Store.
pub struct StoreLocator {
    store: Mutex<Store>,
}

impl StoreLocator {
    pub fn open(dir: &std::path::Path) -> Result<Self, CmdError> {
        Ok(StoreLocator { store: Mutex::new(Store::open_reader(dir)?) })
    }
}

impl FaceLocator for StoreLocator {
    fn locate(&self, hash_hex: &str, face_index: u32) -> Option<PathBuf> {
        let key = FaceKey::from_hex(hash_hex, face_index).ok()?;
        self.store.lock().ok()?.face_path(&key).ok().flatten()
    }
}

pub type Service = PreviewService<StoreLocator>;

/// Mở dịch vụ preview (cache ở `dirs.cache/preview.db`). Gọi sau khi Store đã mở.
pub fn open_service(state: &AppState) -> Result<(), CmdError> {
    std::fs::create_dir_all(&state.dirs.cache).map_err(|e| CmdError::app_io("preview_cache_dir", &e))?;
    let cache = PreviewCache::open(&state.dirs.cache.join("preview.db"), Budgets::DEFAULT)?;
    let locator = StoreLocator::open(&state.dirs.data)?;
    let _ = state.preview.set(Arc::new(PreviewService::new(cache, locator)));
    Ok(())
}

pub fn service(state: &AppState) -> Result<Arc<Service>, CmdError> {
    state.preview.get().cloned().ok_or_else(|| CmdError::new("store_unavailable", "dịch vụ preview chưa mở"))
}

/// Origin được phép đọc phản hồi; bản debug thêm dev server của Vite.
pub fn cors() -> CorsPolicy {
    let c = CorsPolicy::app();
    if cfg!(debug_assertions) { c.with("http://localhost:3000") } else { c }
}

fn unavailable() -> PreviewHttp {
    PreviewHttp {
        status: 503,
        content_type: "text/plain",
        cache_control: "no-store",
        allow_origin: None,
        vary: "Origin",
        body: b"store_unavailable".to_vec(),
    }
}

/// Xử lý một yêu cầu `fontatlas-preview`: Store chưa mở → 503.
pub fn serve_preview(state: &AppState, uri: &str, origin: Option<&str>) -> PreviewHttp {
    match service(state) {
        Ok(svc) => handle_preview_uri(&svc, uri, origin, &cors()),
        Err(_) => unavailable(),
    }
}

pub fn serve_font(state: &AppState, uri: &str, origin: Option<&str>) -> PreviewHttp {
    match service(state) {
        Ok(svc) => handle_font_uri(svc.locator(), uri, origin, &cors()),
        Err(_) => unavailable(),
    }
}

pub fn to_http(p: PreviewHttp) -> tauri::http::Response<Vec<u8>> {
    let mut b = tauri::http::Response::builder()
        .status(p.status)
        .header("Content-Type", p.content_type)
        .header("Cache-Control", p.cache_control)
        .header("Vary", p.vary);
    if let Some(o) = p.allow_origin {
        b = b.header("Access-Control-Allow-Origin", o);
    }
    b.body(p.body).unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
}

pub fn preview_info(state: &AppState) -> Result<PreviewInfoDto, CmdError> {
    let svc = service(state)?;
    Ok(PreviewInfoDto { renderer_version: RENDERER_VERSION.into(), generation: svc.generation(), buckets: PX_BUCKETS.to_vec() })
}

pub fn bump_generation(state: &AppState) -> Result<u64, CmdError> {
    Ok(service(state)?.bump_generation())
}

pub fn prewarm(state: &AppState, items: &[PreviewItemDto]) -> Result<(), CmdError> {
    let svc = service(state)?;
    let reqs: Vec<PreviewRequest> = items
        .iter()
        .map(|i| PreviewRequest { hash_hex: i.hash_hex.clone(), face_index: i.face_index, px: i.px, text: i.text.clone(), generation: Some(i.generation) })
        .collect();
    // lỗi từng mục (thế hệ cũ, file mất…) không làm hỏng cả lô: ảnh sẽ báo lỗi riêng khi được xin
    let _ = svc.prewarm(&reqs);
    Ok(())
}

pub fn cache_info(state: &AppState) -> Result<PreviewCacheInfoDto, CmdError> {
    let (bytes, entries) = service(state)?.cache_stats()?;
    Ok(PreviewCacheInfoDto { bytes, entries })
}

pub fn clear_cache(state: &AppState) -> Result<(), CmdError> {
    Ok(service(state)?.clear_cache()?)
}

const MAX_COVERAGE_CHARS: usize = 512;

/// Grapheme (đã tách ở giao diện) mà font của face không có glyph, theo cmap thật. Rỗng nghĩa là font có đủ.
pub fn missing_chars(state: &AppState, face: &FaceKeyDto, chars: &[String]) -> Result<Vec<String>, CmdError> {
    if chars.len() > MAX_COVERAGE_CHARS {
        return Err(CmdError::invalid_arg(format!("tối đa {MAX_COVERAGE_CHARS} ký tự mỗi lần")));
    }
    let path = service(state)?.locator().locate(&face.hash_hex, face.face_index).ok_or_else(|| CmdError::new("not_found", "không tìm thấy file font"))?;
    let file = fontatlas_core::FontFile::open(&path).map_err(|e| CmdError::new("invalid_font", e.to_string()))?;
    let faces = file.faces().map_err(|e| CmdError::new("invalid_font", e.to_string()))?;
    let f = faces.iter().find(|f| f.index() == face.face_index).ok_or_else(|| CmdError::new("not_found", "không có face này trong file"))?;
    Ok(fontatlas_preview::missing_graphemes(f.font(), chars))
}
