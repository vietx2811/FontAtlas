mod common;
use common::*;
use fontatlas_embed::Whitening;
#[cfg(feature = "fit")]
use fontatlas_embed::fit;

#[cfg(feature = "fit")]
#[test]
fn fit_apply_unit_and_decorrelated() {
    let data = lcg_clusters(3000, 768, 10, 20261018);
    let w = fit(&data, 128, 0.1).unwrap();
    let out: Vec<Vec<f32>> = data.iter().filter_map(|x| w.apply(x)).collect();
    assert_eq!(out.len(), data.len());
    assert!(out.iter().all(|v| v.len() == 128 && (norm(v) - 1.0).abs() < 1e-4));
    assert!(pearson(&col(&out, 0), &col(&out, 1)).abs() < 0.1);
}

#[cfg(feature = "fit")]
#[test]
fn roundtrip_bytes_and_sha() {
    let w = fit(&lcg_clusters(500, 768, 10, 1), 128, 0.1).unwrap();
    let b = w.to_bytes(&serde_json::json!({"version":"white-test","raw_dim":768,"dim":128,"shrink":0.1,"source":"t","split":"tune","n":500}));
    let r = Whitening::from_bytes(&b).unwrap();
    assert_eq!((r.version.as_str(), r.sha256.len()), ("white-test", 64));
    let x = &lcg_clusters(1, 768, 1, 9)[0];
    assert_eq!(w.apply(x), r.apply(x));
}

#[test]
fn rejects_bad_files_wrong_dim_and_zero() {
    assert!(Whitening::from_bytes(b"FAWHITEN\x05\x00\x00\x00{bad}").is_err());
    assert!(Whitening::from_bytes(b"NOTWHITE").is_err());
    let mut short = identity_whitening_bytes(768, 128);
    short.truncate(short.len() - 4);
    assert!(Whitening::from_bytes(&short).is_err()); // thiếu byte của W
    let w = Whitening::from_bytes(&identity_whitening_bytes(768, 128)).unwrap();
    assert_eq!((w.raw_dim, w.dim, w.version.as_str()), (768, 128, "white-identity"));
    assert!(w.apply(&vec![0.0; 768]).is_none());
    assert!(w.apply(&vec![1.0; 512]).is_none());
}

#[test]
fn identity_apply_is_l2_of_first_dims() {
    let w = Whitening::from_bytes(&identity_whitening_bytes(768, 128)).unwrap();
    let mut x = vec![0f32; 768];
    x[0] = 3.0;
    x[1] = 4.0;
    x[700] = 100.0; // ngoài 128 chiều đầu: bị chiếu bỏ
    let v = w.apply(&x).unwrap();
    assert!((v[0] - 0.6).abs() < 1e-6 && (v[1] - 0.8).abs() < 1e-6);
}

#[cfg(feature = "fit")]
#[test]
fn too_few_samples() {
    assert!(fit(&lcg_clusters(100, 768, 3, 3), 128, 0.1).is_err());
}
