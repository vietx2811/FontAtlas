//! fontatlas-eval: bộ đánh giá Similar/Pairing theo phép chia cố định, dựng thư viện tham chiếu.
pub mod classify_gt;
mod corpus;
mod csvio;
mod embeds;
mod metrics;
mod refstats;
mod split;
pub mod taxonomy_v2;

pub use corpus::{Corpus, Origin};
pub use csvio::{
    EvalError, Expected, PairRow, SimilarSet, load_pairs, load_similar, test_only_families,
};
pub use embeds::embed_corpus;
pub use metrics::{
    FamilyRecall, LabelCounts, LabelSource, Outcome, PairCounts, SimilarReport,
    image_only_same_family, markdown, pairing_report, same_family, similar_report,
};
pub use refstats::{build_reference, d_fam, quantiles, spearman};
pub use split::{SIMILAR_SPLIT_SEED, split_similar};
