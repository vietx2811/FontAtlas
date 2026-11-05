//! Trạng thái dùng chung của app: thư mục dữ liệu, hai kết nối Store, từ điển mô tả, kích hoạt.
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use fontatlas_describe::Dictionary;
use fontatlas_store::Store;

use crate::dto::settings::BootStatusDto;
use crate::error::CmdError;
use crate::hub::Hubs;
use crate::services::scan::ScanManager;
use crate::services::activation::ActivationApi;

#[derive(Debug, Clone)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub models: PathBuf,
}

impl AppDirs {
    /// `root` chứa tất cả (dùng cho test và `FONTATLAS_DATA_DIR` ở bản debug).
    pub fn under(root: &std::path::Path) -> Self {
        AppDirs { data: root.to_path_buf(), cache: root.join("cache"), models: root.join("models") }
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
}

impl AppState {
    pub fn new(dirs: AppDirs, activation: Box<dyn ActivationApi>) -> Arc<AppState> {
        Arc::new(AppState {
            dirs,
            store: OnceLock::new(),
            user: OnceLock::new(),
            dictionary: Dictionary::bundled(),
            activation,
            boot: RwLock::new(BootStatusDto::default()),
            hubs: Hubs::default(),
            scans: ScanManager::default(),
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
