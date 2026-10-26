//! fontatlas-index: hồ sơ trục chuẩn hoá theo thư viện tham chiếu, Similar, Pairing.
mod embed_index;
mod explain;
mod index;
mod pair_axes;
mod pairing;
mod profile;
mod reference;
mod similar;
mod superfamily;

pub use pair_axes::{AxisGroup, AxisLabel, PairAxis, Relation, Thresholds, label_axes, same_size};
pub use profile::{AxisId, FaceKey, FaceProfile, Q, Reading, Role};
pub use reference::{AxisQuantiles, IndexError, MIN_SAMPLES, ReferenceTable};

pub const SCORER_VERSION: &str = "pair-0.1.0";
pub use embed_index::{ExactEmbeddings, RERANK_CANDIDATES};
pub use explain::{
    Lang, all_templates, axis_name, explain_pair, explain_similar, fmt_num, pair_axis_name,
};
pub use index::Index;
pub use pairing::{
    Block, BodyWarning, ClashKind, FamilyFacts, Gate, PairAssessment, PairContext, RankKey, Side,
    assess, global_distance, rank_key,
};
pub use similar::{
    EMBED_WEIGHT, Group, GroupWeights, MAX_EMBED_WEIGHT, MIN_SHARED_DIMS, SimilarHit,
    SimilarSource, axis_distance, combine, embed_cos,
};
pub use superfamily::{
    PREFIX_STOP_ALWAYS, PREFIX_STOP_UNLESS_SAME_MANUFACTURER, SuperfamilyTable, same_designer,
};
