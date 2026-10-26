//! Nhúng ảnh mọi face của corpus (không qua store): render render-v2 đúng vị trí trục → encode → whiten.
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use fontatlas_core::{FontFile, standard_location};
use fontatlas_embed::{BATCH, CANVAS, EmbedError, Encode, render_input, to_tensor};
use fontatlas_index::FaceKey;
use rayon::prelude::*;
use skrifa::MetadataProvider;

use crate::corpus::{Corpus, Origin};

const PIXELS: usize = 3 * (CANVAS as usize) * (CANVAS as usize);

/// Tensor đầu vào của một face; `None` khi không đọc/render được (thiếu chữ Latin, file lỗi).
fn tensor_of(o: &Origin) -> Option<Vec<f32>> {
    let file = FontFile::open(&o.path).ok()?;
    let faces = file.faces().ok()?;
    let face = faces.iter().find(|f| f.index() == o.face_index)?;
    let font = face.font();
    let loc = match &o.loc {
        None => standard_location(font),
        Some(l) => font
            .axes()
            .location(l.iter().map(|(t, v)| (t.as_str(), *v))),
    };
    let img = render_input(font, &loc).ok()?;
    let mut t = vec![0f32; PIXELS];
    to_tensor(&img, &mut t);
    Some(t)
}

/// Trả (vector đã whiten theo key, số face bỏ qua). Render song song theo lô để giới hạn RAM.
pub fn embed_corpus(
    c: &Corpus,
    enc: &mut dyn Encode,
    whiten: &dyn Fn(&[f32]) -> Option<Vec<f32>>,
) -> Result<(HashMap<FaceKey, Vec<f32>>, usize), EmbedError> {
    let stop = AtomicBool::new(false);
    let mut out = HashMap::new();
    let mut skipped = 0;
    for chunk in c.profiles.chunks(BATCH * 64) {
        let rendered: Vec<(FaceKey, Option<Vec<f32>>)> = chunk
            .par_iter()
            .map(|p| (p.key.clone(), c.origin(&p.key).and_then(tensor_of)))
            .collect();
        let ok: Vec<(FaceKey, Vec<f32>)> = rendered
            .into_iter()
            .filter_map(|(k, t)| match t {
                Some(t) => Some((k, t)),
                None => {
                    skipped += 1;
                    None
                }
            })
            .collect();
        for b in ok.chunks(BATCH) {
            let ts: Vec<Vec<f32>> = b.iter().map(|x| x.1.clone()).collect();
            for ((k, _), raw) in b.iter().zip(enc.encode(&ts, &stop)?) {
                match whiten(&raw) {
                    Some(w) => {
                        out.insert(k.clone(), w);
                    }
                    None => skipped += 1,
                }
            }
        }
    }
    Ok((out, skipped))
}
