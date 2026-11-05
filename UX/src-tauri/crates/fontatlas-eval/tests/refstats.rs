mod common;
use common::*;
use fontatlas_eval::{d_fam, quantiles, spearman};
use fontatlas_index::{AxisId, Thresholds};

#[test]
fn quantiles_basic() {
    let mut v: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let q = quantiles(&mut v).unwrap();
    assert_eq!(q[0], 0.0);
    assert_eq!(q[100], 999.0);
    assert!((q[50] - 500.0).abs() <= 1.0);
    assert!(quantiles(&mut vec![1.0; 10]).is_none());
}

#[test]
fn spearman_monotone_and_ties() {
    let x: Vec<f32> = (0..50).map(|i| i as f32).collect();
    let y: Vec<f32> = x.iter().map(|v| v * v).collect();
    assert!((spearman(&x, &y).unwrap() - 1.0).abs() < 1e-6);
    let yr: Vec<f32> = x.iter().rev().cloned().collect();
    assert!((spearman(&x, &yr).unwrap() + 1.0).abs() < 1e-6);
    assert!(spearman(&x, &[3.0; 50]).is_none()); // phương sai 0
    assert!(spearman(&x, &y[..40]).is_none()); // lệch độ dài
}

#[test]
fn d_fam_uses_adjacent_upright_weights_only() {
    let mut l = synth_full("Fam", "Light", 0.12);
    l.weight_class = Some(300);
    let mut r = synth_full("Fam", "Regular", 0.15);
    r.weight_class = Some(400);
    let mut b = synth_full("Fam", "Bold", 0.26);
    b.weight_class = Some(700);
    let mut i = synth_full("Fam", "Italic", 0.15);
    i.weight_class = Some(400);
    set_value(&mut i, AxisId::SlantDeg, 12.0);
    let (_, pairs) = d_fam(&[l, r, b, i], &Thresholds::v1(), &test_reference()).unwrap();
    assert_eq!(pairs, 1); // chỉ (300, 400); 400→700 có Δ = 300 > 100; italic bị loại
}
