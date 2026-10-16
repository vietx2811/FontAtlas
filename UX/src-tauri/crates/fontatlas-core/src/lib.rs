//! fontatlas-core: phân tích font thuần, không I/O DB.
mod error;
pub mod geom;
mod load;
mod metadata;
mod metric;

pub use error::CoreError;
pub use load::{Container, Face, FontFile};
pub use metadata::{AxisInfo, LicenseClass, Metadata, OutlineFormat, classify_license};
pub use metric::{FailReason, Metric, Quality};
