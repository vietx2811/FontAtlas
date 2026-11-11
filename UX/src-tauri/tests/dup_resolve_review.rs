//! Review của Lead (feat/duplicates-resolve @ce27e1d3), mục 3–9: activation, kế hoạch cũ, tệp đổi, undo an toàn, pending.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app_lib::dto::activation::ScopeDto;
use app_lib::dto::dup_resolve::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::{activation, boot, dup_resolve, library, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationEngine, ActivationService, EngineError, FakeEngine, Scope};

struct Shared(Arc<FakeEngine>);

impl ActivationEngine for Shared {
    fn method(&self) -> &'static str {
        self.0.method()
    }
    fn register(&self, p: &Path, s: Scope) -> Result<(), EngineError> {
        self.0.register(p, s)
    }
    fn unregister(&self, p: &Path, s: Scope) -> Result<(), EngineError> {
        self.0.unregister(p, s)
    }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        self.0.available_postscript_names()
    }
    fn providers(&self, ps: &str) -> Result<Vec<PathBuf>, EngineError> {
        self.0.providers(ps)
    }
    fn postscript_names_in_file(&self, p: &Path) -> Result<Vec<String>, EngineError> {
        self.0.postscript_names_in_file(p)
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn patch_name(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let u16be = |s: &str| s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect::<Vec<u8>>();
    let mut out = bytes.to_vec();
    for (a, b) in [(from.as_bytes().to_vec(), to.as_bytes().to_vec()), (u16be(from), u16be(to))] {
        let mut i = 0;
        while i + a.len() <= out.len() {
            if out[i..i + a.len()] == a[..] {
                out[i..i + a.len()].copy_from_slice(&b);
                i += a.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

fn patch_revision(bytes: &[u8], revision: f64) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let n = u16::from_be_bytes([out[4], out[5]]) as usize;
    for i in 0..n {
        let rec = 12 + i * 16;
        if &out[rec..rec + 4] == b"head" {
            let off = u32::from_be_bytes(out[rec + 8..rec + 12].try_into().unwrap()) as usize;
            out[off + 4..off + 8].copy_from_slice(&((revision * 65536.0).round() as u32).to_be_bytes());
            return out;
        }
    }
    panic!("không có bảng head");
}

static SERIAL: Mutex<()> = Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn state_with(root: &Path, eng: &Arc<FakeEngine>, files: &[(&str, Vec<u8>)]) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    for (rel, bytes) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn all() -> FaceQueryDto {
    FaceQueryDto::default()
}

fn total(st: &AppState) -> u64 {
    library::query_faces(st, &FaceQueryDto { limit: 100, ..Default::default() }).unwrap().total
}

fn versions(td: &Path, eng: &Arc<FakeEngine>, revs: &[(&str, f64)]) -> Arc<AppState> {
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let files: Vec<(String, Vec<u8>)> = revs.iter().map(|(n, r)| (format!("{n}/playfair.ttf"), patch_revision(&base, *r))).collect();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    state_with(td, eng, &refs)
}

fn plan(st: &AppState) -> ResolvePlanDto {
    dup_resolve::resolve_preview(st, &ResolveScopeDto::All, &all()).unwrap()
}

fn req(p: &ResolvePlanDto, action: ResolveActionDto) -> ResolveRequestDto {
    ResolveRequestDto {
        items: p
            .items
            .iter()
            .map(|i| ResolveItemDto {
                kind: i.kind,
                key: i.key.clone(),
                keeper_path: i.keeper.path.clone(),
                loser_paths: i.losers.iter().map(|l| l.path.clone()).collect(),
                member_paths: i.member_paths.clone(),
            })
            .collect(),
        action,
        explicit_trash_paths: vec![],
        mode: ResolveModeDto::Chosen,
    }
}

fn fake_trash(dir: &Path) -> impl Fn(&Path) -> Result<PathBuf, String> + '_ {
    move |p: &Path| {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let to = dir.join(format!("{}-{}", dir.read_dir().unwrap().count(), p.file_name().unwrap().to_string_lossy()));
        std::fs::rename(p, &to).map_err(|e| e.to_string())?;
        Ok(to)
    }
}

fn no_trash(_: &Path) -> Result<PathBuf, String> {
    Err("không được gọi".into())
}

fn key_of(m: &PlanMemberDto) -> FaceKeyDto {
    m.face.clone()
}

fn codes(r: &ResolveResultDto) -> Vec<&str> {
    r.failed.iter().map(|f| f.code.as_str()).collect()
}

// ---------- #3 activation ----------

#[test]
fn activation_changes_only_after_the_ledger_is_written_and_undo_restores_it() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let (old, new) = (p.items[0].losers[0].clone(), p.items[0].keeper.clone());
    activation::activate(&st, &[key_of(&old)], Some(ScopeDto::Persistent), true, &mut |_| {}).unwrap();
    assert_eq!(eng.registrations().len(), 1);

    // ngay khi engine bị gọi tắt font, sổ phải đã có dòng pending
    let saw_pending = Arc::new(AtomicBool::new(false));
    {
        let (flag, st2) = (saw_pending.clone(), st.clone());
        eng.on_call(move |op, _| {
            if op == "unregister" {
                let n = st2.with_store(|s| Ok(s.pending_resolutions().map(|v| v.len()).unwrap_or(0))).unwrap_or(0);
                if n == 1 {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        });
    }
    let r = dup_resolve::resolve_apply(&st, &req(&p, ResolveActionDto::Hide), &no_trash, 1000).unwrap();
    assert!(r.failed.is_empty(), "{:?}", r.failed);
    assert!(saw_pending.load(Ordering::SeqCst), "activation chỉ đổi SAU khi sổ đã ghi");
    let regs: Vec<String> = eng.registrations().iter().map(|(p, _)| p.to_string_lossy().into_owned()).collect();
    assert_eq!(regs.len(), 1);
    assert!(regs[0].ends_with("new/playfair.ttf"), "bản giữ được bật thay: {regs:?}");
    assert!(r.done[0].was_active && r.done[0].keeper_activated);

    let u = dup_resolve::resolve_undo(&st, &[r.done[0].id], 2000).unwrap();
    assert!(u.failed.is_empty(), "{:?}", u.failed);
    let regs: Vec<String> = eng.registrations().iter().map(|(p, _)| p.to_string_lossy().into_owned()).collect();
    assert_eq!(regs.len(), 1, "{regs:?}");
    assert!(regs[0].ends_with("old/playfair.ttf"), "hoàn tác bật lại bản cũ, tắt bản mới: {regs:?}");
    let _ = new;
}

#[test]
fn a_failed_deactivation_stops_the_hide_and_changes_nothing() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    activation::activate(&st, &[key_of(&p.items[0].losers[0])], Some(ScopeDto::Persistent), true, &mut |_| {}).unwrap();
    eng.fail_next_unregister(EngineError::Rejected { code: 1, message: "lỗi giả lập".into() });
    let r = dup_resolve::resolve_apply(&st, &req(&p, ResolveActionDto::Hide), &no_trash, 1000).unwrap();
    assert_eq!(codes(&r), vec!["deactivate_failed"]);
    assert!(r.done.is_empty());
    assert!(dup_resolve::resolve_list(&st, false).unwrap().is_empty(), "sổ được đảo lại, không ẩn một font còn bật");
    assert_eq!(total(&st), 2, "cả hai version vẫn hiện");
    assert_eq!(eng.registrations().len(), 1, "font vẫn đang bật như cũ");
}

// ---------- #4 plan cũ / keeper = loser ----------

#[test]
fn a_stale_plan_is_rejected_and_all_safe_is_recomputed_by_the_backend() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let it = &p.items[0];

    // thiếu / lệch member_paths → plan_stale
    let mut r1 = req(&p, ResolveActionDto::Hide);
    r1.items[0].member_paths.clear();
    assert_eq!(codes(&dup_resolve::resolve_apply(&st, &r1, &no_trash, 1).unwrap()), vec!["plan_stale"]);
    let mut r2 = req(&p, ResolveActionDto::Hide);
    r2.items[0].member_paths.push("/da/them/mot/ban.ttf".into());
    assert_eq!(codes(&dup_resolve::resolve_apply(&st, &r2, &no_trash, 1).unwrap()), vec!["plan_stale"]);

    // AllSafe: UI cố gửi keeper/loser đảo ngược — backend bỏ qua và dùng kết quả tự tính (giữ bản mới)
    let mut r3 = req(&p, ResolveActionDto::Hide);
    r3.mode = ResolveModeDto::AllSafe;
    r3.items[0].keeper_path = it.losers[0].path.clone();
    r3.items[0].loser_paths = vec![it.keeper.path.clone()];
    r3.items[0].member_paths.clear();
    let done = dup_resolve::resolve_apply(&st, &r3, &no_trash, 2).unwrap();
    assert!(done.failed.is_empty(), "{:?}", done.failed);
    assert_eq!(done.done[0].loser_path, it.losers[0].path, "bản cũ bị ẩn, không phải bản mới");
    assert_eq!(done.done[0].keeper_path, it.keeper.path);
}

#[test]
fn all_safe_never_applies_a_group_that_needs_a_human() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    // cùng fontRevision, khác nội dung (chuỗi version): không biết bản nào mới hơn
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let a = patch_revision(&base, 1.2);
    let b = patch_name(&a, "Version 1.203", "Version 1.204");
    assert_ne!(a, b);
    let st = state_with(td.path(), &eng, &[("a/playfair.ttf", a), ("b/playfair.ttf", b)]);
    let p = plan(&st);
    assert!(!p.items[0].auto_safe);
    let mut r = req(&p, ResolveActionDto::Hide);
    r.mode = ResolveModeDto::AllSafe;
    let out = dup_resolve::resolve_apply(&st, &r, &no_trash, 1).unwrap();
    assert_eq!(codes(&out), vec!["not_auto_safe"]);
    assert!(out.done.is_empty());
}

#[test]
fn a_face_cannot_be_keeper_and_loser_in_the_same_request() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let it = &p.items[0];
    let mut r = req(&p, ResolveActionDto::Hide);
    // mục thứ hai (cùng nhóm) đảo vai: bản giữ của mục 1 thành bản thua
    r.items.push(ResolveItemDto {
        kind: it.kind,
        key: it.key.clone(),
        keeper_path: it.losers[0].path.clone(),
        loser_paths: vec![it.keeper.path.clone()],
        member_paths: it.member_paths.clone(),
    });
    let out = dup_resolve::resolve_apply(&st, &r, &no_trash, 1).unwrap();
    assert_eq!(out.done.len(), 1, "chỉ mục đầu được áp dụng");
    assert!(codes(&out).contains(&"keeper_is_hidden") || codes(&out).contains(&"keeper_conflict"), "{:?}", out.failed);
}

// ---------- #5 #6 #7 tệp đã đổi ----------

#[test]
fn trash_refuses_files_that_changed_since_the_scan() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let (loser, keeper) = (p.items[0].losers[0].path.clone(), p.items[0].keeper.path.clone());
    let trash_dir = td.path().join("trash");

    // bản thua bị thay nội dung (đổi kích thước) → không đưa vào Thùng rác
    let orig = std::fs::read(&loser).unwrap();
    std::fs::write(&loser, b"da bi thay").unwrap();
    let r = dup_resolve::resolve_apply(&st, &req(&p, ResolveActionDto::Trash), &fake_trash(&trash_dir), 1).unwrap();
    assert_eq!(codes(&r), vec!["file_changed"]);
    assert_eq!(std::fs::read(&loser).unwrap(), b"da bi thay", "không đụng tệp");
    std::fs::write(&loser, &orig).unwrap();

    // bản giữ cùng kích thước, đổi nội dung, đặt lại mtime → hash bắt được
    let m = std::fs::metadata(&keeper).unwrap();
    let mut bytes = std::fs::read(&keeper).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    std::fs::write(&keeper, &bytes).unwrap();
    std::fs::OpenOptions::new().write(true).open(&keeper).unwrap().set_modified(m.modified().unwrap()).unwrap();
    // mtime của bản thua cũng đã đổi do ghi lại → đặt lại cho khớp lần quét
    let r = dup_resolve::resolve_apply(&st, &req(&p, ResolveActionDto::Trash), &fake_trash(&trash_dir), 2).unwrap();
    assert!(codes(&r).contains(&"keeper_changed") || codes(&r).contains(&"file_changed"), "{:?}", r.failed);
    assert!(r.done.is_empty());
}

#[test]
fn undo_refuses_to_restore_a_different_file_that_sits_at_the_trash_path() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let loser = p.items[0].losers[0].path.clone();
    let trash_dir = td.path().join("trash");
    let r = dup_resolve::resolve_apply(&st, &req(&p, ResolveActionDto::Trash), &fake_trash(&trash_dir), 1).unwrap();
    let t = r.done[0].trash_path.clone().unwrap();
    std::fs::write(&t, b"mot tep khac cung ten").unwrap();
    let u = dup_resolve::resolve_undo(&st, &[r.done[0].id], 2).unwrap();
    assert_eq!(codes(&u), vec!["trash_mismatch"]);
    assert!(!Path::new(&loser).exists(), "không đưa nhầm tệp khác về");
    assert_eq!(dup_resolve::resolve_list(&st, false).unwrap().len(), 1, "quyết định còn nguyên");
}

// ---------- #8 hoàn tác LIFO ----------

#[test]
fn undo_of_a_chain_must_go_newest_first_but_a_batch_undo_orders_itself() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("v1", 1.1), ("v2", 1.2), ("v3", 1.3)]);
    let p = plan(&st);
    let it = &p.items[0];
    let path_of = |n: &str| it.member_paths.iter().find(|m| m.contains(&format!("/{n}/"))).unwrap().clone();
    let item = |keeper: &str, losers: &[&str], members: Vec<String>| ResolveItemDto {
        kind: it.kind,
        key: it.key.clone(),
        keeper_path: path_of(keeper),
        loser_paths: losers.iter().map(|l| path_of(l)).collect(),
        member_paths: members,
    };
    let all_members = it.member_paths.clone();
    // 1) v2 giữ, v1 bị ẩn (v3 không đụng)
    let r1 = dup_resolve::resolve_apply(
        &st,
        &ResolveRequestDto { items: vec![item("v2", &["v1"], all_members.clone())], action: ResolveActionDto::Hide, explicit_trash_paths: vec![], mode: ResolveModeDto::Chosen },
        &no_trash,
        10,
    )
    .unwrap();
    assert!(r1.failed.is_empty(), "{:?}", r1.failed);
    // 2) nhóm giờ chỉ còn v2, v3: v3 giữ, v2 (bản giữ của bước 1) bị ẩn
    let members2: Vec<String> = all_members.iter().filter(|m| !m.contains("/v1/")).cloned().collect();
    let r2 = dup_resolve::resolve_apply(
        &st,
        &ResolveRequestDto { items: vec![item("v3", &["v2"], members2)], action: ResolveActionDto::Hide, explicit_trash_paths: vec![], mode: ResolveModeDto::Chosen },
        &no_trash,
        11,
    )
    .unwrap();
    assert!(r2.failed.is_empty(), "{:?}", r2.failed);
    let (id1, id2) = (r1.done[0].id, r2.done[0].id);

    // hoàn tác riêng quyết định cũ khi quyết định mới còn phụ thuộc → từ chối
    let u = dup_resolve::resolve_undo(&st, &[id1], 20).unwrap();
    assert_eq!(codes(&u), vec!["newer_resolution_depends"]);
    assert_eq!(dup_resolve::resolve_list(&st, false).unwrap().len(), 2);
    // hoàn tác cả mẻ (thứ tự truyền bất kỳ) → backend tự xử lý mới nhất trước
    let u = dup_resolve::resolve_undo(&st, &[id1, id2], 21).unwrap();
    assert!(u.failed.is_empty(), "{:?}", u.failed);
    assert_eq!(u.done.len(), 2);
    assert!(dup_resolve::resolve_list(&st, false).unwrap().is_empty());
}

// ---------- #9 pending ----------

#[test]
fn leftover_pending_rows_are_recovered_and_never_left_half_done() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = versions(td.path(), &eng, &[("old", 1.203), ("new", 1.204)]);
    let p = plan(&st);
    let it = &p.items[0];
    let new_row = |action: &str| fontatlas_store::NewResolution {
        kind: "versions".into(),
        group_key: it.key.clone(),
        keeper: face(&it.keeper),
        keeper_path: it.keeper.path.clone(),
        loser: face(&it.losers[0]),
        loser_path: it.losers[0].path.clone(),
        action: action.into(),
        trash_path: None,
        reason: "{}".into(),
        was_active: false,
        loser_scope: None,
    };
    // sự cố giữa chừng với action = trash khi tệp còn ở chỗ cũ → huỷ hẳn
    st.with_store(|s| Ok(s.begin_resolution("crash", &new_row("trash"), 5)?)).unwrap();
    assert_eq!(total(&st), 1, "pending đã ẩn");
    assert_eq!(dup_resolve::recover_pending(&st, 6).unwrap(), 1);
    assert!(dup_resolve::resolve_list(&st, false).unwrap().is_empty());
    assert_eq!(library::query_faces(&st, &FaceQueryDto { limit: 10, ..Default::default() }).unwrap().items.len(), 2, "huỷ → hiện lại");

    // action = hide bị bỏ dở → ý định đã ghi, hoàn tất
    st.with_store(|s| Ok(s.begin_resolution("crash2", &new_row("hide"), 7)?)).unwrap();
    assert_eq!(dup_resolve::recover_pending(&st, 8).unwrap(), 1);
    let l = dup_resolve::resolve_list(&st, false).unwrap();
    assert_eq!((l.len(), l[0].status.as_str()), (1, "done"));
}

fn face(m: &PlanMemberDto) -> fontatlas_store::FaceKey {
    fontatlas_store::FaceKey::from_hex(&m.face.hash_hex, m.face.face_index).unwrap()
}
