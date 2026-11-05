//! Superfamily (R2): bảng tay (chỉ từ tập tune) cộng quy tắc metadata — designer, tiền tố tên.
use crate::profile::FaceProfile;
use crate::reference::IndexError;

/// Tiền tố tên quá chung → không bao giờ coi là cùng superfamily.
pub const PREFIX_STOP_ALWAYS: [&str; 6] = ["libre", "google", "red", "zen", "big", "old"];
/// Tiền tố chỉ tính khi cùng manufacturer.
pub const PREFIX_STOP_UNLESS_SAME_MANUFACTURER: [&str; 3] = ["noto", "source", "ibm"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuperfamilyTable {
    /// (nhóm, family viết thường)
    rows: Vec<(String, String)>,
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

impl SuperfamilyTable {
    /// Cột `group,family[,source]`; bỏ dòng `#` và dòng tiêu đề.
    pub fn from_csv(s: &str) -> Result<Self, IndexError> {
        let mut rows = Vec::new();
        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("group,") {
                continue;
            }
            let mut it = line.split(',');
            match (it.next(), it.next()) {
                (Some(g), Some(f)) if !g.trim().is_empty() && !f.trim().is_empty() => {
                    rows.push((norm(g), norm(f)))
                }
                _ => {
                    return Err(IndexError::Reference(format!(
                        "superfamily dòng {}: cần group,family",
                        i + 1
                    )));
                }
            }
        }
        Ok(SuperfamilyTable { rows })
    }

    /// Bảng phát hành kèm crate.
    pub fn bundled() -> Self {
        Self::from_csv(include_str!("../data/superfamilies-v1.csv")).unwrap_or_default()
    }

    fn groups(&self, family: &str) -> impl Iterator<Item = &str> {
        let f = norm(family);
        self.rows
            .iter()
            .filter(move |(_, x)| *x == f)
            .map(|(g, _)| g.as_str())
    }

    pub fn related(&self, a: &FaceProfile, b: &FaceProfile) -> bool {
        let (fa, fb) = (norm(&a.family), norm(&b.family));
        if fa == fb {
            return true;
        }
        if self
            .groups(&a.family)
            .any(|g| self.groups(&b.family).any(|h| h == g))
        {
            return true;
        }
        if same_designer(a, b) {
            return true;
        }
        let (Some(pa), Some(pb)) = (fa.split_whitespace().next(), fb.split_whitespace().next())
        else {
            return false;
        };
        if pa != pb || PREFIX_STOP_ALWAYS.contains(&pa) {
            return false;
        }
        if PREFIX_STOP_UNLESS_SAME_MANUFACTURER.contains(&pa) {
            let ma = a
                .manufacturer
                .as_deref()
                .map(norm)
                .filter(|s| !s.is_empty());
            let mb = b
                .manufacturer
                .as_deref()
                .map(norm)
                .filter(|s| !s.is_empty());
            return ma.is_some() && ma == mb;
        }
        true
    }
}

/// Cùng designer (khác rỗng, không phân biệt hoa thường).
pub fn same_designer(a: &FaceProfile, b: &FaceProfile) -> bool {
    match (
        a.designer.as_deref().map(norm),
        b.designer.as_deref().map(norm),
    ) {
        (Some(x), Some(y)) => !x.is_empty() && x == y,
        _ => false,
    }
}
