//! fontatlas-core: phân tích font thuần, không I/O DB.
mod error;
mod load;

pub use error::CoreError;
pub use load::{Container, Face, FontFile};
