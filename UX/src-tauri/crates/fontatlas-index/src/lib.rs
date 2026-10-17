//! fontatlas-index: hồ sơ trục chuẩn hoá theo thư viện tham chiếu, Similar, Pairing.
mod pair_axes;
mod profile;
mod reference;

pub use pair_axes::{AxisGroup, AxisLabel, PairAxis, Relation, Thresholds, label_axes, same_size};
pub use profile::{AxisId, FaceKey, FaceProfile, Q, Reading, Role};
pub use reference::{AxisQuantiles, IndexError, MIN_SAMPLES, ReferenceTable};

pub const SCORER_VERSION: &str = "pair-0.1.0";
