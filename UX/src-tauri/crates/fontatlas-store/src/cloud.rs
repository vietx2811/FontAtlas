//! Nhận diện nguồn nằm trên ổ cloud theo đường dẫn (macOS File Provider + iCloud Drive).
use std::path::{Component, Path};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    OneDrive,
    GoogleDrive,
    Dropbox,
    Box,
    ICloud,
    /// Thư mục lạ dưới `~/Library/CloudStorage` (File Provider khác).
    Other,
}

impl CloudProvider {
    /// Chuỗi ổn định cho DTO/TS.
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::OneDrive => "one_drive",
            CloudProvider::GoogleDrive => "google_drive",
            CloudProvider::Dropbox => "dropbox",
            CloudProvider::Box => "box",
            CloudProvider::ICloud => "icloud",
            CloudProvider::Other => "other",
        }
    }
}

/// Provider của `path` nếu nằm dưới `…/Library/CloudStorage/<Tên>…` hoặc `…/Library/Mobile Documents/…`.
/// Chỉ nhìn đường dẫn (không chạm đĩa); so khớp theo thành phần, không phân biệt hoa thường.
pub fn cloud_provider_of(path: &Path) -> Option<CloudProvider> {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect();
    let i = parts.iter().position(|p| p == "library")?;
    match parts.get(i + 1).map(String::as_str)? {
        "mobile documents" => Some(CloudProvider::ICloud),
        "cloudstorage" => {
            let name = parts.get(i + 2)?;
            // thư mục File Provider đặt tên "<Provider>[-<tài khoản>]", vd "OneDrive-CongTy", "GoogleDrive-a@b.com"
            let head = name.split('-').next().unwrap_or(name);
            Some(match head {
                "onedrive" => CloudProvider::OneDrive,
                "googledrive" => CloudProvider::GoogleDrive,
                "dropbox" => CloudProvider::Dropbox,
                "box" => CloudProvider::Box,
                _ => CloudProvider::Other,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Option<CloudProvider> {
        cloud_provider_of(Path::new(s))
    }

    #[test]
    fn detects_file_provider_folders() {
        assert_eq!(p("/Users/a/Library/CloudStorage/OneDrive-CongTy/Fonts"), Some(CloudProvider::OneDrive));
        assert_eq!(p("/Users/a/Library/CloudStorage/OneDrive-Personal"), Some(CloudProvider::OneDrive));
        assert_eq!(p("/Users/a/Library/CloudStorage/GoogleDrive-a@b.com/My Drive/Fonts"), Some(CloudProvider::GoogleDrive));
        assert_eq!(p("/Users/a/Library/CloudStorage/Dropbox"), Some(CloudProvider::Dropbox));
        assert_eq!(p("/Users/a/Library/CloudStorage/Box-Box/Fonts"), Some(CloudProvider::Box));
        assert_eq!(p("/Users/a/Library/CloudStorage/SynologyDrive-x"), Some(CloudProvider::Other));
    }

    #[test]
    fn detects_icloud_drive() {
        assert_eq!(
            p("/Users/a/Library/Mobile Documents/com~apple~CloudDocs/Fonts"),
            Some(CloudProvider::ICloud)
        );
        assert_eq!(p("/Users/a/Library/Mobile Documents"), Some(CloudProvider::ICloud));
    }

    #[test]
    fn ignores_local_paths_and_lookalikes() {
        assert_eq!(p("/Users/a/Fonts"), None);
        assert_eq!(p("/Volumes/VXData/Users/VietX/Fonts"), None);
        assert_eq!(p("/Users/a/Library/Fonts"), None);
        assert_eq!(p("/Users/a/Library/CloudStorage"), None, "thiếu thư mục provider");
        // phải đúng vị trí Library/CloudStorage, không khớp rải rác
        assert_eq!(p("/Users/a/Documents/CloudStorage/OneDrive"), None);
        assert_eq!(p("/Users/a/OneDrive/Fonts"), None);
        assert_eq!(p("relative/Library/CloudStorage/Dropbox"), Some(CloudProvider::Dropbox));
    }

    #[test]
    fn case_insensitive_and_serialises_snake_case() {
        assert_eq!(p("/users/a/library/cloudstorage/ONEDRIVE-x"), Some(CloudProvider::OneDrive));
        assert_eq!(serde_json::to_string(&CloudProvider::GoogleDrive).unwrap(), "\"google_drive\"");
        assert_eq!(CloudProvider::GoogleDrive.as_str(), "google_drive");
    }
}
