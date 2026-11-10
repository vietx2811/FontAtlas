//! Trạng thái dùng chung của app: thư mục dữ liệu, hai kết nối Store, từ điển mô tả, kích hoạt, hàng đợi.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use fontatlas_describe::Dictionary;
use fontatlas_embed::{CandleEncoder, EmbedError, Encode, Manifest, bundled_manifest};
use fontatlas_store::Store;

use crate::dto::settings::BootStatusDto;
use crate::error::CmdError;
use crate::hub::Hubs;
use crate::index_bundle::{IndexSlot, RebuildState};
use crate::services::activation::ActivationApi;
use crate::services::embed::EmbedSupervisor;
use crate::services::model::ModelRuntime;
use crate::services::scan::ScanManager;

#[derive(Debug, Clone)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub models: PathBuf,
}

impl AppDirs {
    /// `root` chứa tất cả (dùng cho test và `FONTATLAS_DATA_DIR` ở bản debug).
    pub fn under(root: &Path) -> Self {
        AppDirs { data: root.to_path_buf(), cache: root.join("cache"), models: root.join("models") }
    }
}

/// Tạo encoder + tên thiết bị ("metal" | "cpu") từ file model. Bản thật dùng Candle, test dùng encoder giả.
pub type EncoderFactory = Box<dyn Fn(&Path) -> Result<(Box<dyn Encode>, &'static str), EmbedError> + Send + Sync>;
pub type DiskFreeFn = Box<dyn Fn(&Path) -> std::io::Result<u64> + Send + Sync>;

/// Phần phụ thuộc môi trường, tiêm được trong test.
pub struct Deps {
    pub manifest: Manifest,
    pub encoder: EncoderFactory,
    pub disk_free: DiskFreeFn,
}

impl Deps {
    pub fn production() -> Deps {
        Deps {
            manifest: bundled_manifest(),
            encoder: Box::new(|p| {
                let e = CandleEncoder::load_best(p)?;
                let id = e.device_id();
                Ok((Box::new(e) as Box<dyn Encode>, id))
            }),
            disk_free: Box::new(fontatlas_embed::disk_free),
        }
    }
}

pub struct AppState {
    pub dirs: AppDirs,
    /// Đọc thư viện và service kích hoạt (nhận `&Mutex<Store>`). Đặt một lần khi `Store::open` thành công.
    pub store: OnceLock<Mutex<Store>>,
    /// Kết nối chỉ user.db: mọi ghi dữ liệu người dùng (không tranh khoá với lô ingest).
    pub user: OnceLock<Mutex<Store>>,
    pub dictionary: Dictionary,
    pub activation: Box<dyn ActivationApi>,
    pub boot: RwLock<BootStatusDto>,
    pub hubs: Hubs,
    pub scans: ScanManager,
    pub index: IndexSlot,
    pub rebuild: RebuildState,
    pub deps: Deps,
    pub model: ModelRuntime,
    pub embed: EmbedSupervisor,
    pub langs: crate::services::langs::LangSupervisor,
    pub preview: OnceLock<Arc<crate::services::preview::Service>>,
    /// Máy chủ broker localhost (None khi chưa mở hoặc đã tắt).
    pub broker: Mutex<Option<fontatlas_broker::Server>>,
}

impl AppState {
    pub fn new(dirs: AppDirs, activation: Box<dyn ActivationApi>) -> Arc<AppState> {
        Self::with_deps(dirs, activation, Deps::production())
    }

    pub fn with_deps(dirs: AppDirs, activation: Box<dyn ActivationApi>, deps: Deps) -> Arc<AppState> {
        Arc::new(AppState {
            dirs,
            store: OnceLock::new(),
            user: OnceLock::new(),
            dictionary: Dictionary::bundled(),
            activation,
            boot: RwLock::new(BootStatusDto::default()),
            hubs: Hubs::default(),
            scans: ScanManager::default(),
            index: RwLock::new(None),
            rebuild: RebuildState::default(),
            deps,
            model: ModelRuntime::default(),
            embed: EmbedSupervisor::default(),
            langs: Default::default(),
            preview: OnceLock::new(),
            broker: Mutex::new(None),
        })
    }

    fn unavailable() -> CmdError {
        CmdError::new("store_unavailable", "kho dữ liệu chưa mở được")
    }

    pub fn store(&self) -> Result<&Mutex<Store>, CmdError> {
        self.store.get().ok_or_else(Self::unavailable)
    }

    pub fn user(&self) -> Result<&Mutex<Store>, CmdError> {
        self.user.get().ok_or_else(Self::unavailable)
    }

    /// Khoá ngắn quanh `f` (không giữ qua I/O dài hay callback).
    pub fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, CmdError>) -> Result<T, CmdError> {
        let mut g = self.store()?.lock().map_err(|_| CmdError::internal("khoá store bị hỏng"))?;
        f(&mut g)
    }

    pub fn with_user<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, CmdError>) -> Result<T, CmdError> {
        let mut g = self.user()?.lock().map_err(|_| CmdError::internal("khoá user bị hỏng"))?;
        f(&mut g)
    }
}
