//! fontatlas-activate: kích hoạt / tắt font của thư viện cho mọi app trên máy, không copy file.
//!
//! - [`ActivationEngine`]: lớp nói chuyện với hệ điều hành (CoreText trên macOS; Windows là stub).
//! - [`FakeEngine`]: bản trong bộ nhớ, mô phỏng ngữ nghĩa CoreText đo ở `docs/team/spikes/04_activation.md`.
mod engine;
mod fake;
mod paths;
mod types;

pub use engine::{ActivationEngine, Scope};
pub use fake::FakeEngine;
pub use paths::{FileProbe, canonical_path, classify_owner, probe_file};
pub use types::{ActivationError, EngineError, Owner};
