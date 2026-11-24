//! Lý do auto (`auto:<app>`) không bao giờ gắn lên font mà người dùng đã bật; app thứ hai vẫn thêm lý do riêng cho font chỉ có lý do auto.
mod common;
use common::*;

use fontatlas_activate::{ConflictPolicy, Scope};
use fontatlas_store::{ActivationReason, AutoApp, FaceKey};

const LIBRE: &str = "libre_baskerville.ttf";

fn row(env: &Env, k: &FaceKey) -> fontatlas_store::ActivationRow {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap()
}

#[test]
fn auto_never_attaches_to_a_font_the_user_turned_on() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.activate_faces(&env.store, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    let before = row(&env, &k);
    svc.activate_auto(&env.store, &[k], AutoApp::Illustrator).unwrap();
    let after = row(&env, &k);
    assert_eq!(after.reasons, vec![ActivationReason::Manual], "không gắn auto:illustrator lên font bật tay");
    assert_eq!((after.scope, after.state, after.status, after.updated_at), (before.scope, before.state, before.status, before.updated_at), "không ghi DB");
}

#[test]
fn a_second_app_adds_its_own_auto_reason_to_an_auto_only_font() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.activate_auto(&env.store, &[k], AutoApp::Photoshop).unwrap();
    svc.activate_auto(&env.store, &[k], AutoApp::Illustrator).unwrap();
    let mut got = row(&env, &k).reasons;
    got.sort_by_key(|r| format!("{r:?}"));
    let mut want = vec![ActivationReason::Auto(AutoApp::Photoshop), ActivationReason::Auto(AutoApp::Illustrator)];
    want.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(got, want);
}
