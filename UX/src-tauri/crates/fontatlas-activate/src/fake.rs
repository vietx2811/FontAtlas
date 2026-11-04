//! `FakeEngine`: hệ điều hành giả trong bộ nhớ, đúng ngữ nghĩa CoreText đo ở spike 04:
//! đăng ký trùng (path, scope) → `AlreadyRegistered`; gỡ thứ chưa có → `NotRegistered`;
//! hai đường dẫn khác nhau cùng PostScript name đều đăng ký được.
//!
//! Công khai (không chỉ `cfg(test)`) để test của crate và của lớp lệnh Tauri dùng chung.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier, Mutex, MutexGuard};

use crate::engine::{ActivationEngine, Scope};
use crate::types::EngineError;

#[derive(Default)]
struct FakeState {
    regs: Vec<(PathBuf, Scope)>,
    names: HashMap<PathBuf, Vec<String>>,
    system: Vec<(String, PathBuf)>,
    fail_next: Option<EngineError>,
    panic_on: HashSet<PathBuf>,
    barrier: Option<Arc<Barrier>>,
}

#[derive(Default)]
pub struct FakeEngine {
    st: Mutex<FakeState>,
}

impl FakeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn st(&self) -> MutexGuard<'_, FakeState> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Font "có sẵn" trên máy (hệ thống, app khác) — luôn hiện trong `providers`, không gỡ được.
    pub fn with_system_font(self, ps: &str, path: &Path) -> Self {
        self.st().system.push((ps.to_string(), path.to_path_buf()));
        self
    }

    /// PostScript name mà file ở `path` "chứa".
    pub fn set_names_for(&self, path: &Path, names: &[&str]) {
        self.st().names.insert(path.to_path_buf(), names.iter().map(|s| s.to_string()).collect());
    }

    /// Lần `register` kế tiếp trả lỗi này (một lần).
    pub fn fail_next_register(&self, err: EngineError) {
        self.st().fail_next = Some(err);
    }

    /// Lần `register(path)` kế tiếp panic trước khi ghi nhận — giả app crash (một lần).
    pub fn panic_on_register(&self, path: &Path) {
        self.st().panic_on.insert(path.to_path_buf());
    }

    /// Lần `register` kế tiếp chờ `b.wait()` trước khi chạy (giả đăng ký chậm, một lần).
    pub fn block_register_until(&self, b: Arc<Barrier>) {
        self.st().barrier = Some(b);
    }

    /// Các đăng ký hiện có, theo thứ tự (path, scope).
    pub fn registrations(&self) -> Vec<(PathBuf, Scope)> {
        let mut v = self.st().regs.clone();
        v.sort_by(|a, b| (&a.0, a.1.as_str()).cmp(&(&b.0, b.1.as_str())));
        v
    }

    /// Đăng xuất: mọi đăng ký Session biến mất, Persistent ở lại.
    pub fn simulate_logout(&self) {
        self.st().regs.retain(|(_, s)| *s == Scope::Persistent);
    }
}

impl ActivationEngine for FakeEngine {
    fn method(&self) -> &'static str {
        "fake"
    }

    fn register(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        let barrier = self.st().barrier.take();
        if let Some(b) = barrier {
            b.wait();
        }
        let boom = self.st().panic_on.remove(path);
        if boom {
            panic!("FakeEngine: giả crash khi đăng ký {}", path.display());
        }
        let mut st = self.st();
        if let Some(e) = st.fail_next.take() {
            return Err(e);
        }
        if st.regs.iter().any(|(p, s)| p == path && *s == scope) {
            return Err(EngineError::AlreadyRegistered);
        }
        st.regs.push((path.to_path_buf(), scope));
        Ok(())
    }

    fn unregister(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        let mut st = self.st();
        let before = st.regs.len();
        st.regs.retain(|(p, s)| !(p == path && *s == scope));
        if st.regs.len() == before { Err(EngineError::NotRegistered) } else { Ok(()) }
    }

    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        let st = self.st();
        let mut out: HashSet<String> = st.system.iter().map(|(n, _)| n.clone()).collect();
        for (p, _) in &st.regs {
            out.extend(st.names.get(p).into_iter().flatten().cloned());
        }
        Ok(out)
    }

    fn providers(&self, postscript_name: &str) -> Result<Vec<PathBuf>, EngineError> {
        let st = self.st();
        let mut out: Vec<PathBuf> =
            st.system.iter().filter(|(n, _)| n == postscript_name).map(|(_, p)| p.clone()).collect();
        for (p, _) in &st.regs {
            if st.names.get(p).is_some_and(|ns| ns.iter().any(|n| n == postscript_name)) && !out.contains(p) {
                out.push(p.clone());
            }
        }
        Ok(out)
    }

    fn postscript_names_in_file(&self, path: &Path) -> Result<Vec<String>, EngineError> {
        Ok(self.st().names.get(path).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn fake_mirrors_coretext() {
        let a = Path::new("/lib/a.ttf");
        let b = Path::new("/copy/a.ttf");
        let e = FakeEngine::new().with_system_font("Helvetica", Path::new("/System/Library/Fonts/Helvetica.ttc"));
        e.set_names_for(a, &["A-Regular"]);
        e.set_names_for(b, &["A-Regular"]);
        assert!(e.available_postscript_names().unwrap().contains("Helvetica"));
        assert!(!e.available_postscript_names().unwrap().contains("A-Regular"));

        e.register(a, Scope::Session).unwrap();
        assert_eq!(e.register(a, Scope::Session), Err(EngineError::AlreadyRegistered));
        assert_eq!(e.unregister(a, Scope::Persistent), Err(EngineError::NotRegistered));
        e.register(b, Scope::Persistent).unwrap();
        let mut prov = e.providers("A-Regular").unwrap();
        prov.sort();
        assert_eq!(prov, vec![b.to_path_buf(), a.to_path_buf()]);
        assert!(e.available_postscript_names().unwrap().contains("A-Regular"));
        assert_eq!(e.postscript_names_in_file(a).unwrap(), vec!["A-Regular".to_string()]);

        e.simulate_logout();
        assert_eq!(e.registrations(), vec![(b.to_path_buf(), Scope::Persistent)]);
        e.unregister(b, Scope::Persistent).unwrap();
        assert!(e.registrations().is_empty());
        assert_eq!(e.providers("Helvetica").unwrap().len(), 1);
    }

    #[test]
    fn fake_injected_failure_is_one_shot() {
        let e = FakeEngine::new();
        let p = Path::new("/x.ttf");
        e.fail_next_register(EngineError::Rejected { code: 999, message: "x".into() });
        assert_eq!(e.register(p, Scope::Session), Err(EngineError::Rejected { code: 999, message: "x".into() }));
        e.register(p, Scope::Session).unwrap();
    }

    #[test]
    fn fake_panic_on_register_leaves_no_registration() {
        let e = FakeEngine::new();
        let p = Path::new("/boom.ttf");
        e.panic_on_register(p);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.register(p, Scope::Session)));
        assert!(r.is_err());
        assert!(e.registrations().is_empty());
        e.register(p, Scope::Session).unwrap(); // không bị poison, chỉ panic một lần
    }
}
