//! Index trong bộ nhớ của app: dựng từ kết nối đọc riêng (không giữ khoá Store chính), kèm metadata nhẹ
//! từng face. Dựng lại sau quét/embed có debounce; con trỏ chỉ đổi khi dựng xong.
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use fontatlas_describe::{FaceView, HardConstraint, evaluate};
use fontatlas_embed::{StoreEmbeddingSource, Whitening, bundled_manifest, model_version, raw_version};
use fontatlas_index::{AxisId, ExactEmbeddings, FaceKey as IxKey, FaceProfile, Index, Q, ReferenceTable, SuperfamilyTable};
use fontatlas_store::{FaceKey, Store};
use serde_json::Value;

use crate::dto::library::AxisFilterDto;
use crate::error::CmdError;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct FaceMeta {
    pub face_id: i64,
    pub source_ids: Vec<i64>,
    pub missing: bool,
    pub container: Option<String>,
    pub file_size: u64,
    /// `vi_status` ∈ {full, precomposed_only}
    pub vi_supported: bool,
    /// Font built for a major non-Latin script (CJK, Indic, Thai, Arabic…), per OpenType
    /// script tags in the stored metadata. Its Latin subset is secondary.
    pub non_latin_primary: bool,
    /// Phân loại tự động (snake_case), chưa áp ghi đè của người dùng.
    pub auto_class: String,
    pub auto_subclass: Option<String>,
    /// Tag hình thái tự động (stencil, rounded…).
    pub auto_traits: Vec<String>,
    /// name ID 5 (chuỗi phiên bản).
    pub version: Option<String>,
}

/// OpenType script tags whose presence marks a font as designed primarily for that script.
/// Arabic, Hebrew, Greek, Cyrillic, Georgian, Armenian are deliberately absent: pan-Latin
/// families (macOS Arial, Times New Roman) carry them as secondary scripts.
const NON_LATIN_PRIMARY: &[&str] = &[
    "hani", "kana", "hang", "jamo", "bopo", "deva", "dev2", "beng", "bng2", "guru", "gur2", "gujr", "gjr2",
    "orya", "ory2", "taml", "tml2", "telu", "tel2", "knda", "knd2", "mlym", "mlm2", "sinh", "thai", "lao ",
    "khmr", "mymr", "mym2", "tibt", "ethi", "cans", "cher", "mong",
];
/// AAT-only CJK fonts (Songti, AppleGothic) have no OpenType script list; their glyph count
/// gives them away. Pan-Latin families stay far below (Arial 3381 on macOS).
const CJK_GLYPHS: u64 = 5000;

pub fn is_non_latin_primary(scripts: &[&str], num_glyphs: Option<u64>) -> bool {
    scripts.iter().any(|t| NON_LATIN_PRIMARY.contains(t)) || num_glyphs.is_some_and(|n| n > CJK_GLYPHS)
}

#[cfg(test)]
mod script_tests {
    #[test]
    fn classifies_by_opentype_script_tags() {
        use super::is_non_latin_primary as f;
        assert!(f(&["cyrl", "grek", "hani", "kana", "latn"], Some(20339))); // Hiragino Sans
        assert!(f(&["brah", "gran", "latn", "tml2"], Some(1522))); // Tamil Sangam MN
        assert!(f(&[], Some(8535))); // Songti SC (AAT only)
        assert!(!f(&["arab", "cyrl", "grek", "hebr", "latn"], Some(3381))); // Arial
        assert!(!f(&["latn"], Some(1134))); // Georgia
        assert!(!f(&[], Some(2252))); // Helvetica (AAT only)
    }
}

pub struct IndexBundle {
    pub index: Index,
    pub meta: HashMap<IxKey, FaceMeta>,
    pub by_face_id: HashMap<i64, IxKey>,
    pub model_version: Option<String>,
}

pub fn ix_key(k: &FaceKey) -> IxKey {
    IxKey { hash: k.hash_hex(), index: k.face_index }
}

pub fn store_key(k: &IxKey) -> Option<FaceKey> {
    FaceKey::from_hex(&k.hash, k.index).ok()
}

/// Version của vector đã whiten mà worker embed ghi (khớp `EmbedWorker::model_version`).
pub fn current_model_version() -> String {
    let w = Whitening::bundled();
    model_version(&raw_version(&bundled_manifest().sha256), &w.version, &w.sha256)
}

struct StoreExact(Mutex<StoreEmbeddingSource>);

impl ExactEmbeddings for StoreExact {
    fn fetch(&self, keys: &[IxKey]) -> Vec<Option<Vec<f32>>> {
        let sk: Vec<Option<FaceKey>> = keys.iter().map(store_key).collect();
        let valid: Vec<FaceKey> = sk.iter().flatten().copied().collect();
        let got = match self.0.lock() {
            Ok(src) => src.fetch(&valid).unwrap_or_else(|_| vec![None; valid.len()]),
            Err(_) => vec![None; valid.len()],
        };
        let mut it = got.into_iter();
        sk.iter().map(|k| if k.is_some() { it.next().flatten() } else { None }).collect()
    }
}

impl IndexBundle {
    /// `store` PHẢI là kết nối từ `Store::open_reader` (không bao giờ là `AppState.store`).
    /// Dùng bản phân tích mới nhất có sẵn (bất kể `analyzer_version`).
    pub fn build(store: &Store, model_version: Option<&str>) -> Result<IndexBundle, CmdError> {
        let mut profiles: Vec<FaceProfile> = Vec::new();
        let mut meta: HashMap<IxKey, FaceMeta> = HashMap::new();
        store.for_each_latest_analysis(&mut |a| {
            let parse = |s: &str| serde_json::from_str::<Value>(s).ok();
            let (Some(m), Some(v), Some(an)) = (parse(&a.metadata_json), parse(&a.vietnamese_json), parse(&a.anatomy_json)) else {
                return;
            };
            let hash = a.key.hash_hex();
            let Some(p) = FaceProfile::from_json(&hash, a.key.face_index, &m, &v, &an) else { return };
            let status = v.get("status").and_then(Value::as_str).unwrap_or("");
            let auto = fontatlas_core::classify::classify(&fontatlas_core::classify::ClassifyInput::from_json(&m, &an));
            meta.insert(
                p.key.clone(),
                FaceMeta {
                    auto_class: auto.class.snake().into(),
                    auto_subclass: auto.subclass,
                    auto_traits: auto.traits.clone(),
                    version: m.get("version").and_then(Value::as_str).map(String::from),
                    face_id: a.face_id,
                    source_ids: a.source_ids,
                    missing: a.missing,
                    container: a.container,
                    file_size: a.file_size,
                    vi_supported: matches!(status, "full" | "precomposed_only"),
                    non_latin_primary: is_non_latin_primary(
                        &["scripts", "cmap_scripts"]
                            .iter()
                            .filter_map(|k| m.get(*k).and_then(Value::as_array))
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>(),
                        m.get("num_glyphs").and_then(Value::as_u64),
                    ),
                },
            );
            profiles.push(p);
        })?;
        let mut used_version = None;
        if let Some(ver) = model_version {
            let mut embs: HashMap<IxKey, Vec<f32>> = HashMap::new();
            store.for_each_embedding(ver, &mut |k, v| {
                embs.insert(ix_key(&k), v.to_vec());
            })?;
            if !embs.is_empty() {
                for p in &mut profiles {
                    p.embedding = embs.remove(&p.key);
                }
                used_version = Some(ver.to_string());
            }
        }
        let by_face_id = meta.iter().map(|(k, m)| (m.face_id, k.clone())).collect();
        let mut index = Index::new(profiles, ReferenceTable::bundled(), SuperfamilyTable::bundled());
        if let Some(ver) = &used_version
            && let Ok(src) = StoreEmbeddingSource::open(store.dir(), ver)
        {
            index.set_exact_source(Box::new(StoreExact(Mutex::new(src))));
        }
        Ok(IndexBundle { index, meta, by_face_id, model_version: used_version })
    }

    pub fn len(&self) -> usize {
        self.meta.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
    }

    /// (face_id, điểm) của mọi face qua bộ lọc; `None` = không lọc.
    pub fn scope_for(&self, filters: &[AxisFilterDto], hard: &[HardConstraint]) -> Option<Vec<(i64, f32)>> {
        if filters.is_empty() && hard.is_empty() {
            return None;
        }
        let fl = describe_filters(filters);
        let r = self.index.reference();
        let out = self
            .index
            .profiles()
            .iter()
            .filter_map(|p| {
                let m = self.meta.get(&p.key)?;
                let view = ProfileView { p, r, vi_supported: m.vi_supported };
                Some((m.face_id, evaluate(&fl, hard, &view)?))
            })
            .collect();
        Some(out)
    }

    /// Face có qua các bộ lọc `filters` (chỉ ràng buộc `Required`) và `hard` không.
    pub fn passes(&self, p: &FaceProfile, filters: &[fontatlas_describe::AxisFilter], hard: &[HardConstraint]) -> bool {
        let Some(m) = self.meta.get(&p.key) else { return false };
        let view = ProfileView { p, r: self.index.reference(), vi_supported: m.vi_supported };
        evaluate(filters, hard, &view).is_some()
    }
}

/// `AxisFilterDto` → bộ lọc của `fontatlas-describe`.
pub fn describe_filters(filters: &[AxisFilterDto]) -> Vec<fontatlas_describe::AxisFilter> {
    filters
        .iter()
        .map(|f| fontatlas_describe::AxisFilter {
            axis: f.axis.clone(),
            range: f.range.map(|r| (r[0], r[1])),
            equals: f.equals.clone(),
            strength: match f.strength {
                crate::dto::library::StrengthDto::Required => fontatlas_describe::Strength::Required,
                crate::dto::library::StrengthDto::Preferred => fontatlas_describe::Strength::Preferred,
            },
            weight: f.weight,
            term_id: String::new(),
        })
        .collect()
}

pub struct ProfileView<'a> {
    pub p: &'a FaceProfile,
    pub r: &'a ReferenceTable,
    pub vi_supported: bool,
}

impl FaceView for ProfileView<'_> {
    fn percentile(&self, axis: &str) -> Option<f32> {
        let a = AxisId::ALL.iter().copied().find(|a| a.key() == axis)?;
        let rd = self.p.get(a);
        if rd.q == Q::Failed {
            return None;
        }
        let v = rd.value?;
        match a {
            AxisId::StressDeg | AxisId::SlantDeg => self.r.magnitude_percentile(a, &rd.method, v),
            // serif_foot là lớp, không phải phân vị
            AxisId::SerifFoot => None,
            _ => self.r.percentile(a, &rd.method, v),
        }
    }

    fn class(&self, axis: &str) -> Option<String> {
        match axis {
            "serif_foot" => {
                let rd = self.p.get(AxisId::SerifFoot);
                let v = if rd.q == Q::Failed { return None } else { rd.value? };
                Some(if v < 1.2 { "sans" } else if v > 1.6 { "serif" } else { "uncertain" }.into())
            }
            "a_storeys" | "g_storeys" => {
                let rd = if axis == "a_storeys" { &self.p.a_storeys } else { &self.p.g_storeys };
                (rd.q != Q::Failed).then_some(rd.value).flatten().map(|v| format!("{}", v.round() as i64))
            }
            _ => None,
        }
    }

    fn vi_missing(&self) -> usize {
        self.p.vi_missing
    }

    fn supports_vi(&self) -> bool {
        self.vi_supported
    }
}

// ---- dựng lại có debounce ----

#[derive(Debug, Clone, Copy)]
pub enum RebuildWhy {
    ScanIdle,
    QueueEmpty,
    /// Mỗi 512 face embed xong: chỉ chạy nếu đã đủ `min_interval` từ lần dựng trước.
    EmbedProgress,
}

pub struct RebuildPolicy {
    pub min_interval: Duration,
}

pub struct RebuildState {
    pub policy: Mutex<RebuildPolicy>,
    last: Mutex<Option<Instant>>,
    building: AtomicBool,
    dirty: AtomicBool,
    /// Số lần dựng xong (để test).
    pub builds: std::sync::atomic::AtomicU64,
}

impl Default for RebuildState {
    fn default() -> Self {
        RebuildState {
            policy: Mutex::new(RebuildPolicy { min_interval: Duration::from_secs(30) }),
            last: Mutex::new(None),
            building: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            builds: Default::default(),
        }
    }
}

pub type IndexSlot = RwLock<Option<Arc<IndexBundle>>>;

/// Dựng bằng kết nối đọc riêng, ngoài mọi khoá; lỗi giữ bundle cũ. Đồng bộ.
pub fn rebuild_index(state: &Arc<AppState>) -> Result<(), CmdError> {
    let reader = Store::open_reader(&state.dirs.data)?;
    let bundle = IndexBundle::build(&reader, Some(&current_model_version()))?;
    if let Ok(mut slot) = state.index.write() {
        *slot = Some(Arc::new(bundle));
    }
    state.rebuild.builds.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut l) = state.rebuild.last.lock() {
        *l = Some(Instant::now());
    }
    Ok(())
}

/// ScanIdle/QueueEmpty: dựng ngay (nền). EmbedProgress: chỉ khi ≥ `min_interval` từ lần dựng trước.
/// Một lần dựng tại một thời điểm; yêu cầu đến khi đang dựng chỉ đặt cờ dirty, chạy lại đúng một lần.
pub fn request_rebuild(state: &Arc<AppState>, why: RebuildWhy) {
    if matches!(why, RebuildWhy::EmbedProgress) {
        let min = state.rebuild.policy.lock().map(|p| p.min_interval).unwrap_or_default();
        let recent = state.rebuild.last.lock().ok().and_then(|l| *l).is_some_and(|t| t.elapsed() < min);
        if recent {
            return;
        }
    }
    if state.rebuild.building.swap(true, Ordering::SeqCst) {
        state.rebuild.dirty.store(true, Ordering::SeqCst);
        return;
    }
    let st = Arc::clone(state);
    let spawned = std::thread::Builder::new().name("fontatlas-index".into()).spawn(move || {
        loop {
            let _ = rebuild_index(&st);
            if !st.rebuild.dirty.swap(false, Ordering::SeqCst) {
                st.rebuild.building.store(false, Ordering::SeqCst);
                // yêu cầu lọt vào giữa lúc kết thúc
                if st.rebuild.dirty.load(Ordering::SeqCst) && !st.rebuild.building.swap(true, Ordering::SeqCst) {
                    st.rebuild.dirty.store(false, Ordering::SeqCst);
                    continue;
                }
                break;
            }
        }
    });
    if spawned.is_err() {
        state.rebuild.building.store(false, Ordering::SeqCst);
    }
}

pub fn wait_rebuilt(state: &AppState, timeout: Duration) -> bool {
    let t = Instant::now();
    while state.rebuild.building.load(Ordering::SeqCst) {
        if t.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}
