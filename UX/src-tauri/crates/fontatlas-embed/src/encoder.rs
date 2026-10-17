//! Encoder ảnh qua ONNX Runtime (`ort`), lô tĩnh 4, pad lô cuối, huỷ giữa lúc chạy.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ort::{
    environment::Environment,
    session::{RunOptions, Session, builder::GraphOptimizationLevel},
    value::TensorRef,
};

use crate::{BATCH, CANVAS, EmbedError, RAW_DIM};

const PIXELS: usize = 3 * (CANVAS as usize) * (CANVAS as usize);

/// Giao diện encoder để worker tiêm được bản giả/chậm trong test.
pub trait Encode: Send {
    /// 1..=BATCH tensor CHW → cùng số vector thô.
    fn encode(&mut self, tensors: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn raw_dim(&self) -> usize;
}

fn env() -> Result<&'static Environment, EmbedError> {
    static ENV: OnceLock<Environment> = OnceLock::new();
    if let Some(e) = ENV.get() {
        return Ok(e);
    }
    let e = ort::init().with_name("fontatlas-embed").build().map_err(|e| EmbedError::Model(e.to_string()))?;
    Ok(ENV.get_or_init(|| e))
}

pub struct OrtEncoder {
    session: Session,
}

fn shape_of(o: &ort::value::Outlet) -> Option<Vec<i64>> {
    o.dtype().tensor_shape().map(|s| s.to_vec())
}

impl OrtEncoder {
    pub fn load(path: &Path, intra_threads: usize) -> Result<Self, EmbedError> {
        let m = |e: ort::Error| EmbedError::Model(e.to_string());
        let session = Session::builder(env()?)
            .map_err(m)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| EmbedError::Model(e.to_string()))?
            .with_intra_threads(intra_threads.max(1))
            .map_err(|e| EmbedError::Model(e.to_string()))?
            .commit_from_file(path)
            .map_err(m)?;
        let want_in = vec![BATCH as i64, 3, CANVAS as i64, CANVAS as i64];
        let want_out = vec![BATCH as i64, RAW_DIM as i64];
        let ok = session.inputs().first().and_then(shape_of) == Some(want_in)
            && session.outputs().first().and_then(shape_of) == Some(want_out);
        if !ok {
            return Err(EmbedError::Model("shape".into()));
        }
        Ok(OrtEncoder { session })
    }
}

impl Encode for OrtEncoder {
    fn raw_dim(&self) -> usize {
        RAW_DIM
    }

    fn encode(&mut self, tensors: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        if tensors.is_empty() {
            return Ok(Vec::new());
        }
        if tensors.len() > BATCH || tensors.iter().any(|t| t.len() != PIXELS) {
            return Err(EmbedError::Model("batch".into()));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        // Pad bằng ảnh trắng: (1 − 0.5)/0.5 = 1.0.
        let mut input = vec![1.0f32; BATCH * PIXELS];
        for (i, t) in tensors.iter().enumerate() {
            input[i * PIXELS..(i + 1) * PIXELS].copy_from_slice(t);
        }
        let ro = Arc::new(RunOptions::new().map_err(|e| EmbedError::Model(e.to_string()))?);
        let done = AtomicBool::new(false);
        let session = &mut self.session;
        let result = std::thread::scope(|sc| {
            // Luồng canh cờ huỷ: gọi terminate() trong lúc run đang chạy.
            let ro2 = ro.clone();
            let done_ref = &done;
            sc.spawn(move || {
                while !done_ref.load(Ordering::SeqCst) {
                    if cancel.load(Ordering::SeqCst) {
                        let _ = ro2.terminate();
                        return;
                    }
                    std::thread::park_timeout(Duration::from_millis(20));
                }
            });
            let r = (|| {
                let x = TensorRef::from_array_view(([BATCH, 3usize, CANVAS as usize, CANVAS as usize], &input[..]))?;
                let out = session.run_with_options(ort::inputs![x], &*ro)?;
                let (_, d) = out[0].try_extract_tensor::<f32>()?;
                Ok::<Vec<f32>, ort::Error>(d.to_vec())
            })();
            done.store(true, Ordering::SeqCst);
            r
        });
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        let flat = result.map_err(|e| EmbedError::Model(e.to_string()))?;
        if flat.len() != BATCH * RAW_DIM {
            return Err(EmbedError::Model("output".into()));
        }
        Ok(flat.chunks(RAW_DIM).take(tensors.len()).map(|c| c.to_vec()).collect())
    }
}
