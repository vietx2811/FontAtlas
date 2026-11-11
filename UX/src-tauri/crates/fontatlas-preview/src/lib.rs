//! fontatlas-preview: vẽ preview PNG alpha-mask cho lưới thư viện.
//!
//! Giới hạn (có chủ đích, spec mục 7.1):
//! - Không shaping: chỉ cmap + advance; không kerning, không GSUB/GPOS, không bidi,
//!   không nối chữ Ả Rập/Ấn. Dấu tiếng Việt chỉ đúng với ký tự dựng sẵn (NFC).
//! - Font variable được vẽ ở instance mặc định.
//! - Font bitmap/màu (sbix, CBDT) không có outline → `no_outlines`.
mod cache;
pub mod coverage;
mod error;
mod http;
mod service;
pub mod render;
mod text;

pub use cache::{Budgets, CacheKey, Pool, PreviewCache, cache_key, pool_for};
pub use error::PreviewError;
pub use http::{CorsPolicy, FONT_SCHEME, PREVIEW_SCHEME, PreviewHttp, handle_font_uri, handle_preview_uri, parse_font_uri, parse_preview_uri};
pub use service::{FaceLocator, PreviewRequest, PreviewService};
pub use coverage::{missing_graphemes, missing_set, supported_langs};
pub use render::{Rendered, cmap_sample, render_face};
pub use text::{TextSource, canvas_size, check_px, normalize_text, pick_text};

/// Tăng khi đổi cách vẽ: khoá cache và URL (`r=`) đổi theo.
pub const RENDERER_VERSION: &str = "preview-0.1.0";
pub const PX_BUCKETS: [u32; 4] = [32, 48, 64, 96];
pub const DEFAULT_PX: u32 = 48;
pub const DEFAULT_SAMPLE: &str = "Hamburgefonts Ăn phở";
pub const LATIN_FALLBACK: &str = "Hamburgefonts";
pub const MAX_TEXT_CHARS: usize = 64;
pub const CMAP_SAMPLE_CHARS: usize = 12;
