//! Encoder ảnh bằng candle: tháp vision SigLIP (có head MAP) = `get_image_features`, 768 chiều.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::siglip;

use crate::{BATCH, CANVAS, EmbedError, Encode, RAW_DIM};

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
}

fn m(e: candle_core::Error) -> EmbedError {
    EmbedError::Model(e.to_string())
}

impl CandleEncoder {
    pub fn load(path: &Path, backend: Backend) -> Result<Self, EmbedError> {
        let (device, id) = match backend {
            Backend::Cpu => (Device::Cpu, "cpu"),
            Backend::Metal => (Device::new_metal(0).map_err(m)?, "metal"),
        };
        // SAFETY: mmap file model chỉ đọc; file đã kiểm sha256 và không bị ghi trong lúc app chạy.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[path], DType::F32, &device) }.map_err(m)?;
        let cfg = siglip::Config::base_patch16_224().vision_config;
        let model = siglip::VisionModel::new(&cfg, true, vb.pp("vision_model")).map_err(m)?;
        Ok(CandleEncoder { model, device, id })
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
        let flat: Vec<f32> = tensors.concat();
        let x = Tensor::from_vec(flat, (n, 3, CANVAS as usize, CANVAS as usize), &self.device).map_err(m)?;
        let y = self.model.forward(&x).map_err(m)?;
        let out: Vec<Vec<f32>> = y.to_dtype(DType::F32).map_err(m)?.to_vec2().map_err(m)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        if out.len() != n || out.iter().any(|v| v.len() != RAW_DIM) {
            return Err(EmbedError::Model("đầu ra sai kích thước".into()));
        }
        Ok(out)
    }

    fn raw_dim(&self) -> usize {
        RAW_DIM
    }
}
