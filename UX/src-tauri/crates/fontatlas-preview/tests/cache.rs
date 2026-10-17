use fontatlas_preview::*;

fn k(i: u32) -> CacheKey { cache_key(&"ab".repeat(32), i, 48, "Ăn") }
fn b(d: u64, c: u64) -> Budgets { Budgets { default_bytes: d, custom_bytes: c } }

#[test]
fn round_trip_and_miss() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("previews.db"), Budgets::DEFAULT).unwrap();
    assert_eq!(c.get(&k(0)).unwrap(), None);
    c.put(&k(0), Pool::Custom, b"png0").unwrap();
    assert_eq!(c.get(&k(0)).unwrap().as_deref(), Some(&b"png0"[..]));
    assert_eq!((c.total_bytes(Pool::Custom), c.total_bytes(Pool::Default)), (4, 0));
}

#[test]
fn key_covers_every_component_and_folds_nfc() {
    let h = "ab".repeat(32);
    let nfc = cache_key(&h, 0, 48, "\u{1EAE}n");          // "Ắn" dựng sẵn
    let nfd = cache_key(&h, 0, 48, "A\u{0306}\u{0301}n"); // cùng chữ, dạng tách dấu
    assert_eq!(nfc, nfd, "NFD và NFC cùng khoá");
    assert_ne!(nfc, cache_key(&"cd".repeat(32), 0, 48, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 1, 48, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 0, 64, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 0, 48, "\u{1EAE}m"));
}

#[test]
fn pool_for_default_sample_only() {
    assert_eq!(pool_for(DEFAULT_SAMPLE), Pool::Default);
    assert_eq!(pool_for("  "), Pool::Default); // rỗng → mặc định
    assert_eq!(pool_for("Khác"), Pool::Custom);
}

#[test]
fn eviction_keeps_total_under_budget() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(1 << 20, 10_000)).unwrap();
    for i in 0..3 { c.put(&k(i), Pool::Custom, &vec![1u8; 3000]).unwrap(); }
    assert!(c.get(&k(0)).unwrap().is_some()); // làm mới k(0)
    for i in 3..5 { c.put(&k(i), Pool::Custom, &vec![1u8; 3000]).unwrap(); }
    assert!(c.total_bytes(Pool::Custom) <= 10_000);
    assert!(c.get(&k(0)).unwrap().is_some(), "vừa dùng → giữ");
    assert!(c.get(&k(1)).unwrap().is_none(), "cũ nhất → bị đẩy");
    assert!(c.get(&k(4)).unwrap().is_some());
}

#[test]
fn custom_churn_keeps_default_pool() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(10_000, 1_000)).unwrap();
    c.put(&k(0), Pool::Default, &[1; 500]).unwrap();
    for i in 1..50 { c.put(&k(i), Pool::Custom, &[2; 400]).unwrap(); }
    assert!(c.get(&k(0)).unwrap().is_some(), "chữ tuỳ chỉnh không được đẩy chữ mặc định");
    assert!(c.total_bytes(Pool::Custom) <= 1_000);
}

#[test]
fn oversized_entry_not_stored() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(1 << 20, 1000)).unwrap();
    c.put(&k(0), Pool::Custom, &vec![1u8; 500]).unwrap();
    c.put(&k(1), Pool::Custom, &vec![1u8; 5000]).unwrap();
    assert!(c.get(&k(1)).unwrap().is_none());
    assert!(c.get(&k(0)).unwrap().is_some(), "entry quá lớn không được đẩy entry khác");
}

#[test]
fn put_same_key_replaces_and_recounts() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), Budgets::DEFAULT).unwrap();
    c.put(&k(0), Pool::Custom, &[1; 100]).unwrap();
    c.put(&k(0), Pool::Custom, &[2; 40]).unwrap();
    assert_eq!(c.total_bytes(Pool::Custom), 40);
    assert_eq!(c.len().unwrap(), 1);
}

#[test]
fn reopen_keeps_entries_and_totals_and_applies_smaller_budget() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("p.db");
    { let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap(); for i in 0..10 { c.put(&k(i), Pool::Default, &[1; 100]).unwrap(); } }
    let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap();
    assert_eq!(c.total_bytes(Pool::Default), 1000);
    assert!(c.get(&k(9)).unwrap().is_some());
    drop(c);
    let c = PreviewCache::open(&p, b(450, 450)).unwrap();
    assert!(c.total_bytes(Pool::Default) <= 450);
}

#[test]
fn corrupt_cache_is_renamed_and_recreated() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("previews.db");
    std::fs::write(&p, b"rac rac rac rac rac rac").unwrap();
    let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap();
    assert_eq!(c.len().unwrap(), 0);
    c.put(&k(0), Pool::Custom, b"x").unwrap();
    assert!(c.get(&k(0)).unwrap().is_some());
    let broken: Vec<_> = std::fs::read_dir(d.path()).unwrap()
        .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("previews.broken-") && n.ends_with(".db")).collect();
    assert_eq!(broken.len(), 1, "{broken:?}");
}
