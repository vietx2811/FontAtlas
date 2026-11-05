//! Giao diện encoder (bản thật: `CandleEncoder`).
use std::sync::atomic::AtomicBool;

use crate::EmbedError;

/// Giao diện encoder để worker tiêm được bản giả/chậm trong test.
pub trait Encode: Send {
    /// 1..=BATCH tensor CHW → cùng số vector thô.
    fn encode(&mut self, tensors: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn raw_dim(&self) -> usize;
}
