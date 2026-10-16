//! fontatlas-core: phân tích font thuần, không I/O DB.
mod error;
mod load;
mod metadata;

pub use error::CoreError;
pub use load::{Container, Face, FontFile};
pub use metadata::{AxisInfo, LicenseClass, Metadata, OutlineFormat, classify_license};
