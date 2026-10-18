//! fontatlas-eval: bộ đánh giá Similar/Pairing theo phép chia cố định, dựng thư viện tham chiếu.
mod corpus;
mod csvio;
mod metrics;
mod refstats;

pub use corpus::Corpus;
pub use csvio::{
    EvalError, Expected, PairRow, SimilarSet, load_pairs, load_similar, test_only_families,
};
pub use metrics::{
    LabelCounts, LabelSource, Outcome, PairCounts, SimilarReport, markdown, pairing_report,
    similar_report,
};
pub use refstats::{build_reference, d_fam, quantiles, spearman};
