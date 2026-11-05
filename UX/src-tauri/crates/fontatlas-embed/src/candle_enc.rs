//! Encoder ảnh bằng candle: tháp vision SigLIP (có head MAP) = `get_image_features`.
//! Huỷ: candle không ngắt được giữa một forward, nên cờ huỷ được kiểm trước và sau mỗi lô (≤ BATCH ảnh);
//! độ trễ huỷ tối đa một lô (~0,1 s).
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::siglip;

use crate::{BATCH, CANVAS, EmbedError, Encode};

const PIXELS: usize = 3 * (CANVAS as usize) * (CANVAS as usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cpu,
    Metal,
}

pub struct CandleEncoder {
    model: siglip::VisionModel,
    device: Device,
    id: &'static str,
    raw_dim: usize,
}

fn m(e: candle_core::Error) -> EmbedError {
    EmbedError::Model(e.to_string())
}

fn metal_works() -> bool {
    let Ok(d) = Device::new_metal(0) else { return false };
    Tensor::ones((4, 4), DType::F32, &d)
        .and_then(|t| t.matmul(&t))
        .and_then(|t| t.sum_all())
        .and_then(|t| t.to_scalar::<f32>())
        .is_ok_and(|v| v == 64.0)
}

/// Metal trên macOS nếu khởi tạo và chạy được một phép nhân nhỏ; còn lại CPU. Kết quả được nhớ trong tiến trình.
pub fn select_backend() -> Backend {
    static B: OnceLock<Backend> = OnceLock::new();
    *B.get_or_init(|| if cfg!(target_os = "macos") && metal_works() { Backend::Metal } else { Backend::Cpu })
}

impl CandleEncoder {
    /// Model SigLIP 2 base patch16-224.
    pub fn load(path: &Path, backend: Backend) -> Result<Self, EmbedError> {
        Self::load_with(path, backend, &siglip::Config::base_patch16_224().vision_config)
    }

    /// Thiết bị tốt nhất; Metal nạp lỗi hoặc chạy thử hỏng (kernel lỗi, NaN) thì dùng CPU.
    pub fn load_best(path: &Path) -> Result<Self, EmbedError> {
        match select_backend() {
            Backend::Metal => Self::load(path, Backend::Metal)
                .and_then(Self::verified)
                .or_else(|_| Self::load(path, Backend::Cpu)),
            Backend::Cpu => Self::load(path, Backend::Cpu),
        }
    }

    /// Chạy thử một ảnh trắng qua đủ mọi kernel (conv, attention, gelu, layernorm, head MAP);
    /// đầu ra phải đúng số chiều và hữu hạn.
    pub fn verified(mut self) -> Result<Self, EmbedError> {
        let out = self.encode(&[vec![1.0; PIXELS]], &AtomicBool::new(false))?;
        if out.len() != 1 || out[0].iter().any(|x| !x.is_finite()) {
            return Err(EmbedError::Model(format!("chạy thử trên {} cho kết quả không hợp lệ", self.id)));
        }
        Ok(self)
    }

    pub fn load_with(path: &Path, backend: Backend, cfg: &siglip::VisionConfig) -> Result<Self, EmbedError> {
        let (device, id) = match backend {
            Backend::Cpu => (Device::Cpu, "cpu"),
            Backend::Metal => (Device::new_metal(0).map_err(m)?, "metal"),
        };
        // SAFETY: mmap chỉ sống trong lúc nạp: `VisionModel::new` chép trọng số vào tensor của thiết bị và `vb`
        // (cùng mmap) bị huỷ khi hàm trả về. `ModelStore::ensure` không bao giờ ghi đè tại chỗ (ghi `.part` rồi rename).
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[path], DType::F32, &device) }.map_err(m)?;
        let model = siglip::VisionModel::new(cfg, true, vb.pp("vision_model")).map_err(m)?;
        Ok(CandleEncoder { model, device, id, raw_dim: cfg.hidden_size })
    }

    pub fn device_id(&self) -> &'static str {
        self.id
    }
}

impl Encode for CandleEncoder {
    fn encode(&mut self, tensors: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        if tensors.is_empty() || tensors.len() > BATCH || tensors.iter().any(|t| t.len() != PIXELS) {
            return Err(EmbedError::Model("tensor đầu vào sai kích thước".into()));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        let n = tensors.len();
        let x = Tensor::from_vec(tensors.concat(), (n, 3, CANVAS as usize, CANVAS as usize), &self.device).map_err(m)?;
        let out: Vec<Vec<f32>> = self.model.forward(&x).and_then(|y| y.to_vec2()).map_err(m)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        if out.len() != n || out.iter().any(|v| v.len() != self.raw_dim) {
            return Err(EmbedError::Model("đầu ra sai kích thước".into()));
        }
        Ok(out)
    }

    fn raw_dim(&self) -> usize {
        self.raw_dim
    }
}
