//! `MacCoreTextEngine`: đăng ký font tại chỗ bằng CoreText (`CTFontManager*ForURL`).
//! Hành vi đo ở `docs/team/spikes/04_activation.md` (macOS 27.2): đăng ký trùng → 105,
//! gỡ thứ chưa đăng ký → 201, trùng PostScript name không bị chặn.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};

use objc2_core_foundation::{CFArray, CFDictionary, CFError, CFRetained, CFString, CFURL};
use objc2_core_text::{
    CTFontDescriptor, CTFontManagerCopyAvailablePostScriptNames, CTFontManagerCreateFontDescriptorsFromURL,
    CTFontManagerRegisterFontsForURL, CTFontManagerScope, CTFontManagerUnregisterFontsForURL, kCTFontNameAttribute,
    kCTFontURLAttribute,
};

use crate::engine::{ActivationEngine, Scope};
use crate::paths::canonical_path;
use crate::types::EngineError;

const CT_ERROR_DOMAIN: &str = "com.apple.CoreText.CTFontManagerErrorDomain";
const CT_ALREADY_REGISTERED: i64 = 105;
const CT_NOT_REGISTERED: i64 = 201;

#[derive(Debug, Clone, Copy, Default)]
pub struct MacCoreTextEngine;

fn ct_scope(scope: Scope) -> CTFontManagerScope {
    match scope {
        Scope::Session => CTFontManagerScope::Session,
        Scope::Persistent => CTFontManagerScope::Persistent,
    }
}

fn file_url(path: &Path) -> Result<CFRetained<CFURL>, EngineError> {
    CFURL::from_file_path(path)
        .ok_or_else(|| EngineError::Rejected { code: -1, message: format!("không tạo được URL: {}", path.display()) })
}

/// Ánh xạ `CFError` (đã retain) sang `EngineError`. 105/201 chỉ khi đúng domain CoreText.
fn map_error(err: *mut CFError) -> EngineError {
    let Some(nn) = NonNull::new(err) else {
        return EngineError::Rejected { code: -1, message: "CoreText trả false mà không có CFError".into() };
    };
    // SAFETY: CTFontManager*ForURL trả CFError đã retain qua out-param (quy tắc Create của CF).
    let e = unsafe { CFRetained::from_raw(nn) };
    let domain = e.domain().map(|d| d.to_string()).unwrap_or_default();
    let code = e.code() as i64;
    if domain == CT_ERROR_DOMAIN {
        match code {
            CT_ALREADY_REGISTERED => return EngineError::AlreadyRegistered,
            CT_NOT_REGISTERED => return EngineError::NotRegistered,
            _ => {}
        }
    }
    let desc = e.description().map(|d| d.to_string()).unwrap_or_default();
    EngineError::Rejected { code, message: format!("{domain}: {desc}") }
}

/// PostScript name của một descriptor.
fn ps_name(d: &CTFontDescriptor) -> Option<String> {
    // SAFETY: kCTFontNameAttribute là hằng CoreText hợp lệ; giá trị (nếu có) là CFString.
    let v = unsafe { d.attribute(kCTFontNameAttribute) }?;
    v.downcast_ref::<CFString>().map(|s| s.to_string())
}

fn descriptor_url_path(d: &CTFontDescriptor) -> Option<PathBuf> {
    // SAFETY: kCTFontURLAttribute là hằng CoreText hợp lệ; giá trị (nếu có) là CFURL.
    let v = unsafe { d.attribute(kCTFontURLAttribute) }?;
    v.downcast_ref::<CFURL>().and_then(|u| u.to_file_path())
}

fn descriptors(arr: &CFArray) -> Vec<CFRetained<CTFontDescriptor>> {
    // SAFETY: mọi hàm CoreText dùng ở đây trả CFArray chỉ chứa CTFontDescriptor.
    let arr: &CFArray<CTFontDescriptor> = unsafe { arr.cast_unchecked() };
    (0..arr.len()).filter_map(|i| arr.get(i)).collect()
}

impl ActivationEngine for MacCoreTextEngine {
    fn method(&self) -> &'static str {
        "coretext"
    }

    fn register(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        let url = file_url(path)?;
        let mut err: *mut CFError = ptr::null_mut();
        // SAFETY: `url` sống suốt lời gọi; `err` là con trỏ hợp lệ tới out-param.
        let ok = unsafe { CTFontManagerRegisterFontsForURL(&url, ct_scope(scope), &mut err) };
        if ok { Ok(()) } else { Err(map_error(err)) }
    }

    fn unregister(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        let url = file_url(path)?;
        let mut err: *mut CFError = ptr::null_mut();
        // SAFETY: như `register`.
        let ok = unsafe { CTFontManagerUnregisterFontsForURL(&url, ct_scope(scope), &mut err) };
        if ok { Ok(()) } else { Err(map_error(err)) }
    }

    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        // SAFETY: hàm không tham số, trả CFArray<CFString> đã retain.
        let arr = unsafe { CTFontManagerCopyAvailablePostScriptNames() };
        // SAFETY: tài liệu CoreText: mảng chỉ chứa CFString.
        let arr: &CFArray<CFString> = unsafe { arr.cast_unchecked() };
        Ok((0..arr.len()).filter_map(|i| arr.get(i).map(|s| s.to_string())).collect())
    }

    fn providers(&self, postscript_name: &str) -> Result<Vec<PathBuf>, EngineError> {
        let name = CFString::from_str(postscript_name);
        // SAFETY: kCTFontNameAttribute là hằng CoreText hợp lệ.
        let key: &CFString = unsafe { kCTFontNameAttribute };
        let attrs = CFDictionary::<CFString, CFString>::from_slices(&[key], &[&name]);
        // SAFETY: `attrs` là dict CFString → CFString hợp lệ cho CTFontDescriptorCreateWithAttributes.
        let desc = unsafe { CTFontDescriptor::with_attributes(attrs.as_ref()) };
        // Không đặt mandatory set: lọc lại theo tên chính xác ngay dưới đây cho cùng kết quả.
        // SAFETY: `desc` hợp lệ; None = không có thuộc tính bắt buộc.
        let Some(found) = (unsafe { desc.matching_font_descriptors(None) }) else {
            return Ok(vec![]);
        };
        let mut out: Vec<PathBuf> = vec![];
        for d in descriptors(&found) {
            if ps_name(&d).as_deref() != Some(postscript_name) {
                continue;
            }
            if let Some(p) = descriptor_url_path(&d) {
                let p = canonical_path(&p).unwrap_or(p);
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        Ok(out)
    }

    fn postscript_names_in_file(&self, path: &Path) -> Result<Vec<String>, EngineError> {
        let url = file_url(path)?;
        // SAFETY: `url` hợp lệ; trả None nếu file không phải font.
        let Some(arr) = (unsafe { CTFontManagerCreateFontDescriptorsFromURL(&url) }) else {
            return Ok(vec![]);
        };
        Ok(descriptors(&arr).iter().filter_map(|d| ps_name(d)).collect())
    }
}
