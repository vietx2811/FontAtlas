//! Đọc vector đã lưu (kết nối chỉ đọc riêng, chạy song song với writer): nạp index và rerank f32.
use std::path::Path;

use fontatlas_store::{EmbedReader, FaceKey};

use crate::EmbedError;

pub struct StoreEmbeddingSource {
    reader: EmbedReader,
    model_version: String,
}

impl StoreEmbeddingSource {
    pub fn open(dir: &Path, model_version: &str) -> Result<Self, EmbedError> {
        Ok(StoreEmbeddingSource { reader: EmbedReader::open(dir)?, model_version: model_version.to_string() })
    }

    /// Mọi vector của `model_version` (để gán `FaceProfile.embedding`).
    pub fn load_all(&self) -> Result<Vec<(FaceKey, Vec<f32>)>, EmbedError> {
        let mut out = Vec::new();
        self.reader.for_each_embedding(&self.model_version, &mut |k, v| out.push((k, v.to_vec())))?;
        Ok(out)
    }

    /// Vector theo thứ tự `keys`; `None` khi face chưa có vector của version này.
    pub fn fetch(&self, keys: &[FaceKey]) -> Result<Vec<Option<Vec<f32>>>, EmbedError> {
        Ok(self.reader.embeddings_f32(&self.model_version, keys)?)
    }
}
