//! fontatlas-index: hồ sơ trục chuẩn hoá theo thư viện tham chiếu, Similar, Pairing.
mod pair_axes;
mod pairing;
mod profile;
mod reference;
mod superfamily;

pub use pair_axes::{AxisGroup, AxisLabel, PairAxis, Relation, Thresholds, label_axes, same_size};
pub use profile::{AxisId, FaceKey, FaceProfile, Q, Reading, Role};
pub use reference::{AxisQuantiles, IndexError, MIN_SAMPLES, ReferenceTable};

pub const SCORER_VERSION: &str = "pair-0.1.0";
pub use pairing::{
    Block, BodyWarning, ClashKind, FamilyFacts, Gate, PairAssessment, PairContext, Side, assess,
    global_distance, rank_key,
};
pub use superfamily::{
    PREFIX_STOP_ALWAYS, PREFIX_STOP_UNLESS_SAME_MANUFACTURER, SuperfamilyTable, same_designer,
};
