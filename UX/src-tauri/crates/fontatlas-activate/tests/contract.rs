//! Hợp đồng IPC: JSON do serde thật sinh ra phải khớp fixture. Plan 6b đọc đúng các file này
//! cho kiểu TS. Đổi kiểu → test này đỏ; cập nhật bằng `UPDATE_FIXTURES=1`.
use std::path::PathBuf;

use fontatlas_activate::{
    ActivationReport, ActivationStateDto, AutoDeactivated, Conflict, FileOutcome, FileResult, Owner,
    ReconcileReport, Scope,
};
use fontatlas_store::{ActivationReason, ActivationStatus, DesiredState};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn check(name: &str, json: String) {
    let p = fixture(name);
    if std::env::var("UPDATE_FIXTURES").as_deref() == Ok("1") {
        std::fs::write(&p, format!("{json}\n")).unwrap();
    }
    let want = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("thiếu {name}; chạy với UPDATE_FIXTURES=1"));
    assert_eq!(json.trim_end(), want.trim_end(), "{name} lệch hợp đồng");
}

fn result(i: u8, outcome: FileOutcome) -> FileResult {
    FileResult {
        hash_hex: format!("{i:02x}").repeat(32),
        path: Some(format!("/Users/u/Fonts/f{i}.ttf")),
        requested_faces: vec![0],
        extra_faces: if i == 1 { vec![1, 2] } else { vec![] },
        outcome,
    }
}

#[test]
fn activation_report_json_matches_contract_fixture() {
    let conflict = Conflict {
        postscript_name: "LibreBaskerville-Regular".into(),
        existing_path: "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/L/x.ttf"
            .into(),
        owner: Owner::RightFont,
    };
    let outcomes = vec![
        FileOutcome::Ready,
        FileOutcome::Activated,
        FileOutcome::AlreadyActive,
        FileOutcome::Deactivated,
        FileOutcome::AlreadyInactive,
        FileOutcome::Busy,
        FileOutcome::NeedsConfirmation { conflicts: vec![conflict] },
        FileOutcome::HeldByCollections { collection_ids: vec![3, 7] },
        FileOutcome::KeptActive { manual: true, collection_ids: vec![7] },
        FileOutcome::MissingFile,
        FileOutcome::NotDownloaded,
        FileOutcome::Failed { code: "engine.rejected".into(), message: "hệ điều hành từ chối (999): x".into() },
    ];
    let rep = ActivationReport {
        results: outcomes.into_iter().enumerate().map(|(i, o)| result(i as u8, o)).collect(),
        changed: 2,
        needs_confirmation: 1,
        failed: 1,
    };
    let json = serde_json::to_string_pretty(&rep).unwrap();
    let compact = serde_json::to_string(&rep).unwrap();
    for key in ["\"hashHex\"", "\"requestedFaces\"", "\"extraFaces\"", "\"existingPath\"", "\"collectionIds\"", "\"needsConfirmation\""] {
        assert!(compact.contains(key), "thiếu {key}");
    }
    assert!(compact.contains(r#""kind":"needs_confirmation""#));
    assert!(compact.contains(r#""owner":"right_font""#));
    check("activation_report.json", json);
}

#[test]
fn activation_state_json_matches_contract_fixture() {
    let dto = vec![ActivationStateDto {
        hash_hex: "ab".repeat(32),
        state: DesiredState::Active,
        scope: Scope::Session,
        status: ActivationStatus::Registered,
        reasons: vec![ActivationReason::Manual, ActivationReason::Collection(3)],
        active_path: Some("/Users/u/Fonts/Phở Đậm.ttf".into()),
        conflict_ps: None,
        last_error: None,
    }];
    let compact = serde_json::to_string(&dto).unwrap();
    assert!(compact.contains(r#""kind":"collection","collectionId":3"#));
    assert!(compact.contains(r#""activePath""#) && compact.contains(r#""conflictPs""#));
    check("activation_state.json", serde_json::to_string_pretty(&dto).unwrap());
}

#[test]
fn reconcile_report_json_matches_contract_fixture() {
    let rep = ReconcileReport {
        auto_deactivated: vec![AutoDeactivated {
            hash_hex: "cd".repeat(32),
            faces: vec![0],
            removed_reasons: vec![ActivationReason::Collection(5)],
            cause: "collection_deleted".into(),
        }],
        auto_deactivate_failed: vec![AutoDeactivated {
            hash_hex: "ef".repeat(32),
            faces: vec![0, 1],
            removed_reasons: vec![],
            cause: "no_reason".into(),
        }],
        reregistered: 3,
        moved: 1,
        missing: 1,
        not_downloaded: 0,
        cleaned: 1,
        failed: 0,
        results: vec![result(9, FileOutcome::Activated)],
    };
    let compact = serde_json::to_string(&rep).unwrap();
    assert!(compact.contains(r#""autoDeactivated""#) && compact.contains(r#""autoDeactivateFailed""#));
    assert!(compact.contains(r#""removedReasons""#));
    check("reconcile_report.json", serde_json::to_string_pretty(&rep).unwrap());
}
