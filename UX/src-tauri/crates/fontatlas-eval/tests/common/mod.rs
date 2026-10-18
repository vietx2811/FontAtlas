#![allow(dead_code)]
use std::path::{Path, PathBuf};

#[path = "../../../fontatlas-index/tests/common/mod.rs"]
mod index_common;
pub use index_common::*;

use fontatlas_eval::Corpus;
use fontatlas_index::{Index, SuperfamilyTable};

pub fn core_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures")
}

pub fn eval_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../docs/team/eval")
}

/// Số dòng dữ liệu = dòng không rỗng, không '#', trừ header.
pub fn data_lines(p: &Path) -> usize {
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .count()
        - 1
}

pub fn index_with_test_reference(c: &Corpus) -> Index {
    Index::new(
        c.profiles.clone(),
        test_reference(),
        SuperfamilyTable::bundled(),
    )
}
