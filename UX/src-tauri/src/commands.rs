//! Lệnh Tauri: lớp mỏng, chỉ chuyển tham số rồi gọi `services::*`.
//!
//! `command_list!` là nguồn duy nhất cho `invoke_handler`, bảng `COMMANDS` và bản đồ TypeScript.
//! Mọi lệnh đụng Store/Index/embed là `async` (chạy trong `spawn_blocking`); lệnh `sync` chỉ cho
//! danh sách `SYNC_OK`.
use std::sync::Arc;

use tauri::State;

use tauri::ipc::Channel;

use crate::dto::activation::*;
use crate::dto::auto::*;
use crate::dto::analysis::*;
use crate::dto::cases::*;
use crate::dto::dup_resolve::*;
use crate::dto::events::{ActivationEvent, BootEvent, EmbedEvent, ModelEvent, ScanEvent};
use crate::dto::langs::*;
use crate::dto::library::*;
use crate::dto::model::*;
use crate::dto::preview::*;
use crate::dto::settings::FontManagerDto;
use crate::dto::suggest::*;
use crate::dto::settings::{BootStatusDto, LegacyStateDto, OnboardingDto};
use crate::error::CmdError;
use crate::services::{
    activation, analysis, auto, boot, brief, cases, compare, describe, dup_resolve, duplicates, embed, langs, library, model, onboarding, pairs,
    preview,
    settings, sources, suggest, suggest_pair, system,
};
use crate::state::AppState;

pub struct CommandSig {
    pub name: &'static str,
    pub args: &'static [(&'static str, &'static str)],
    pub result: &'static str,
    pub is_async: bool,
}

/// Lệnh được phép đồng bộ: không đụng Store/Index/embed.
pub const SYNC_OK: &[&str] = &[
    "startup_status",
    "scan_status",
    "preview_info",
    "activation_caps",
];

/// Danh sách lệnh (nguồn duy nhất). Cú pháp mỗi dòng: `async|sync tên(đối số: kiểu, …) -> kiểu_kết_quả;`
/// Gọi `command_list!(__command_table)` để sinh `COMMANDS`, `command_list!(__command_handler)` cho handler.
#[macro_export]
macro_rules! command_list {
    ($cb:ident) => {
        $crate::$cb! {
            sync startup_status() -> Result<BootStatusDto, CmdError>;
            async boot_subscribe(on_event: Channel<BootEvent>) -> Result<(), CmdError>;
            async onboarding_state() -> Result<OnboardingDto, CmdError>;
            async set_onboarding_step(step: Option<String>) -> Result<(), CmdError>;
            async legacy_import() -> Result<LegacyStateDto, CmdError>;
            async legacy_retry() -> Result<LegacyStateDto, CmdError>;
            async list_sources() -> Result<Vec<SourceDto>, CmdError>;
            async add_source(path: String) -> Result<SourceDto, CmdError>;
            async remove_source(source_id: i64) -> Result<RemoveSourceReportDto, CmdError>;
            async remove_source_preview(source_id: i64) -> Result<RemoveSourceReportDto, CmdError>;
            async rescan_sources(source_ids: Option<Vec<i64>>) -> Result<(), CmdError>;
            async scan_pause() -> Result<(), CmdError>;
            async scan_resume() -> Result<(), CmdError>;
            async scan_cancel() -> Result<(), CmdError>;
            sync scan_status() -> Result<ScanStatusDto, CmdError>;
            async scan_subscribe(on_event: Channel<ScanEvent>) -> Result<(), CmdError>;
            async query_faces(q: FaceQueryDto) -> Result<FacePageDto, CmdError>;
            async family_groups(q: FaceQueryDto) -> Result<Vec<FamilyGroupDto>, CmdError>;
            async get_family_styles(family: String) -> Result<Vec<FamilyStyleDto>, CmdError>;
            async set_favorite(face: FaceKeyDto, on: bool) -> Result<(), CmdError>;
            async list_collections() -> Result<Vec<CollectionDto>, CmdError>;
            async create_collection(name: String, color: Option<String>) -> Result<CollectionDto, CmdError>;
            async rename_collection(id: i64, name: String) -> Result<(), CmdError>;
            async delete_collection(id: i64) -> Result<(), CmdError>;
            async add_to_collection(id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async remove_from_collection(id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async list_tags() -> Result<Vec<TagDto>, CmdError>;
            async system_tag_catalog() -> Result<Vec<SystemTagGroupDto>, CmdError>;
            async tags_of(face: FaceKeyDto) -> Result<Vec<TagDto>, CmdError>;
            async create_tag(name: String) -> Result<TagDto, CmdError>;
            async rename_tag(id: i64, name: String) -> Result<TagDto, CmdError>;
            async delete_tag(id: i64) -> Result<u32, CmdError>;
            async tag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async untag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async list_problems() -> Result<Vec<ProblemDto>, CmdError>;
            async retry_problems() -> Result<u32, CmdError>;
            async reveal_in_finder(face: FaceKeyDto) -> Result<(), CmdError>;
            async backup_user_db() -> Result<String, CmdError>;
            async disk_free(target: String) -> Result<u64, CmdError>;
            async get_ui_state() -> Result<UiStateDto, CmdError>;
            async set_ui_state(state: UiStateDto) -> Result<(), CmdError>;
            async get_setting(key: String) -> Result<Option<String>, CmdError>;
            async set_setting(key: String, value: String) -> Result<(), CmdError>;
            async detect_font_managers() -> Result<Vec<FontManagerDto>, CmdError>;
            async get_face(face: FaceKeyDto, lang: LangDto) -> Result<FaceDetailDto, CmdError>;
            async similar(face: FaceKeyDto, opts: SimilarOptsDto) -> Result<Vec<SimilarHitDto>, CmdError>;
            async similar_families(face: FaceKeyDto, opts: SimilarOptsDto, q: FaceQueryDto) -> Result<SimilarFamilyPageDto, CmdError>;
            async axis_stats(q: FaceQueryDto) -> Result<Vec<AxisStatsDto>, CmdError>;
            async axis_histogram(q: FaceQueryDto, bins: u32) -> Result<Vec<AxisHistogramDto>, CmdError>;
            async class_facets(q: FaceQueryDto) -> Result<Vec<ClassFacetDto>, CmdError>;
            async pair_candidates(face: FaceKeyDto, opts: PairOptsDto) -> Result<Vec<PairCandidateDto>, CmdError>;
            async assess_pair(a: FaceKeyDto, b: FaceKeyDto, opts: PairOptsDto) -> Result<PairCandidateDto, CmdError>;
            async role_fit(face: FaceKeyDto, role: RoleRefDto) -> Result<RoleFitDto, CmdError>;
            async role_fit_faces(role: RoleRefDto, opts: RoleFitListOptsDto) -> Result<RoleFitPageDto, CmdError>;
            async suggest_partners(anchor: FaceKeyDto, req: SuggestPartnersReqDto) -> Result<PartnerPageDto, CmdError>;
            async desk_suggest(req: DeskSuggestReqDto) -> Result<DeskPageDto, CmdError>;
            async suggest_combos(req: SuggestCombosReqDto) -> Result<Vec<ComboSuggestionDto>, CmdError>;
            async record_view(face: FaceKeyDto) -> Result<(), CmdError>;
            async recent_views(limit: u32) -> Result<Vec<RecentViewDto>, CmdError>;
            async clear_recent_views() -> Result<u32, CmdError>;
            async save_png(suggested_name: String, png_base64: String) -> Result<Option<String>, CmdError>;
            async duplicate_groups(q: FaceQueryDto) -> Result<Vec<DuplicateGroupDto>, CmdError>;
            async duplicate_summary(q: FaceQueryDto) -> Result<DuplicateSummaryDto, CmdError>;
            async resolve_preview(scope: ResolveScopeDto, q: FaceQueryDto) -> Result<ResolvePlanDto, CmdError>;
            async resolve_apply(req: ResolveRequestDto) -> Result<ResolveResultDto, CmdError>;
            async resolve_undo(ids: Vec<i64>) -> Result<ResolveResultDto, CmdError>;
            async resolve_list(include_undone: bool) -> Result<Vec<ResolutionDto>, CmdError>;
            async duplicate_ignore(kind: DuplicateKindDto, key: String) -> Result<(), CmdError>;
            async duplicate_unignore(kind: DuplicateKindDto, key: String) -> Result<(), CmdError>;
            async duplicate_ignored() -> Result<Vec<IgnoredGroupDto>, CmdError>;
            async source_priority() -> Result<Vec<i64>, CmdError>;
            async set_source_priority(ids: Vec<i64>) -> Result<Vec<i64>, CmdError>;
            async reveal_location(path: String) -> Result<(), CmdError>;
            async language_list(q: FaceQueryDto) -> Result<LangListDto, CmdError>;
            async lang_coverage(face_id: i64, code: String) -> Result<LangCoverageDto, CmdError>;
            async compare_faces(faces: Vec<FaceKeyDto>, opts: CompareOptsDto) -> Result<CompareDto, CmdError>;
            async pair_these(faces: Vec<FaceKeyDto>, opts: PairTheseOptsDto) -> Result<Vec<PairComboDto>, CmdError>;
            async save_pair(pair: NewPairDto) -> Result<i64, CmdError>;
            async list_pairs(collection_id: Option<i64>) -> Result<Vec<SavedPairDto>, CmdError>;
            async delete_pair(id: i64) -> Result<(), CmdError>;
            async describe(text: String) -> Result<DescribeDto, CmdError>;
            async create_case(name: Option<String>, brief: Option<BriefDto>, settings: Option<CaseSettingsDto>) -> Result<CaseDto, CmdError>;
            async list_cases(include_deleted: bool) -> Result<Vec<CaseSummaryDto>, CmdError>;
            async open_case(id: i64) -> Result<CaseDto, CmdError>;
            async get_case(id: i64) -> Result<CaseDto, CmdError>;
            async update_case(id: i64, patch: CasePatchDto) -> Result<CaseDto, CmdError>;
            async delete_case(id: i64) -> Result<(), CmdError>;
            async restore_case(id: i64) -> Result<CaseDto, CmdError>;
            async add_to_case(case_id: Option<i64>, faces: Vec<FaceKeyDto>, origin: Option<CaseOriginDto>) -> Result<AddToCaseDto, CmdError>;
            async remove_from_case(case_id: i64, faces: Vec<FaceKeyDto>) -> Result<u32, CmdError>;
            async create_combo(case_id: i64, name: Option<String>, from_combo: Option<i64>) -> Result<ComboDto, CmdError>;
            async update_combo(id: i64, patch: ComboPatchDto) -> Result<ComboDto, CmdError>;
            async delete_combo(id: i64) -> Result<(), CmdError>;
            async reorder_combos(case_id: i64, ids: Vec<i64>) -> Result<(), CmdError>;
            async set_combo_slot(combo_id: i64, role: ComboRoleDto, face: Option<FaceKeyDto>, note: Option<String>, style: Option<serde_json::Value>) -> Result<ComboDto, CmdError>;
            async add_combo_role(combo_id: i64, name: String) -> Result<ComboDto, CmdError>;
            async remove_combo_role(combo_id: i64, role: ComboRoleDto) -> Result<ComboDto, CmdError>;
            async rename_combo_role(combo_id: i64, role: ComboRoleDto, new_name: String) -> Result<ComboDto, CmdError>;
            async reorder_combo_roles(combo_id: i64, roles: Vec<ComboRoleDto>) -> Result<ComboDto, CmdError>;
            async create_combos(case_id: i64, combos: Vec<NewComboDto>) -> Result<Vec<ComboDto>, CmdError>;
            async case_compare(case_id: i64, combo_ids: Vec<i64>, lang: LangDto) -> Result<CaseCompareDto, CmdError>;
            async brief_starter(brief: BriefDto, case_id: Option<i64>) -> Result<BriefStarterDto, CmdError>;
            async open_case_from_peek(peek: PeekOpenDto) -> Result<PeekOpenResultDto, CmdError>;
            async set_classification_override(over: ClassificationOverrideDto) -> Result<(), CmdError>;
            async clear_classification_override(face: FaceKeyDto) -> Result<(), CmdError>;
            async get_settings() -> Result<SettingsDto, CmdError>;
            async set_settings(patch: SettingsPatchDto) -> Result<SettingsDto, CmdError>;
            async model_status() -> Result<ModelStatusDto, CmdError>;
            async model_download(on_event: Channel<ModelEvent>) -> Result<(), CmdError>;
            async model_cancel() -> Result<(), CmdError>;
            async model_redownload(on_event: Channel<ModelEvent>) -> Result<(), CmdError>;
            async model_remove() -> Result<(), CmdError>;
            async embed_status() -> Result<EmbedStatusDto, CmdError>;
            async embed_pause() -> Result<(), CmdError>;
            async embed_resume() -> Result<(), CmdError>;
            async embed_subscribe(on_event: Channel<EmbedEvent>) -> Result<(), CmdError>;
            async embed_failures() -> Result<Vec<EmbedFailureDto>, CmdError>;
            async queue_pause_all() -> Result<(), CmdError>;
            async queue_resume_all() -> Result<(), CmdError>;
            async activation_state() -> Result<Vec<ActivationStateDto>, CmdError>;
            async activation_preflight(faces: Vec<FaceKeyDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_activate(faces: Vec<FaceKeyDto>, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_replace(faces: Vec<FaceKeyDto>, scope: Option<ScopeDto>, confirmed: Vec<ConfirmedConflictDto>, on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_list_replaced() -> Result<Vec<ReplacedEntryDto>, CmdError>;
            async activation_restore_replaced(id: i64) -> Result<RestoreOutcomeDto, CmdError>;
            async activation_activate_case(case_id: i64, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> Result<CaseActivationDto, CmdError>;
            async activation_activate_collection(collection_id: i64, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_deactivate(faces: Vec<FaceKeyDto>, force: bool, on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_deactivate_collection(collection_id: i64, on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_deactivate_all(on_progress: Channel<ProgressDto>) -> Result<ActivationReportDto, CmdError>;
            async activation_set_keep(on: bool) -> Result<(), CmdError>;
            async activation_availability(faces: Vec<FaceKeyDto>) -> Result<Vec<AvailabilityDto>, CmdError>;
            async activation_set_scope(faces: Vec<FaceKeyDto>, scope: ScopeDto) -> Result<ActivationReportDto, CmdError>;
            async activation_subscribe(on_event: Channel<ActivationEvent>) -> Result<(), CmdError>;
            sync preview_info() -> Result<PreviewInfoDto, CmdError>;
            async preview_bump_generation() -> Result<u64, CmdError>;
            async preview_prewarm(items: Vec<PreviewItemDto>) -> Result<(), CmdError>;
            async missing_chars(face: FaceKeyDto, chars: Vec<String>) -> Result<Vec<String>, CmdError>;
            async preview_cache_info() -> Result<PreviewCacheInfoDto, CmdError>;
            async clear_preview_cache() -> Result<(), CmdError>;
            sync activation_caps() -> Result<ActivationCapsDto, CmdError>;
            async auto_status() -> Result<AutoStatusDto, CmdError>;
            async auto_set_enabled(on: bool) -> Result<AutoStatusDto, CmdError>;
            async auto_install(target: AutoTargetDto) -> Result<AutoStatusDto, CmdError>;
            async auto_remove(target: AutoTargetDto) -> Result<AutoStatusDto, CmdError>;
            async auto_log(limit: Option<u32>) -> Result<Vec<AutoLogDto>, CmdError>;
            async auto_forget_key() -> Result<AutoStatusDto, CmdError>;
            async auto_probe() -> Result<AutoStatusDto, CmdError>;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_table {
    (@async async) => { true };
    (@async sync) => { false };
    ($( $kind:ident $name:ident ( $( $arg:ident : $ty:ty ),* ) -> $ret:ty ; )*) => {
        pub const COMMANDS: &[$crate::commands::CommandSig] = &[
            $( $crate::commands::CommandSig {
                name: stringify!($name),
                args: &[ $( (stringify!($arg), stringify!($ty)) ),* ],
                result: stringify!($ret),
                is_async: $crate::__command_table!(@async $kind),
            } ),*
        ];
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_handler {
    ($( $kind:ident $name:ident ( $( $arg:ident : $ty:ty ),* ) -> $ret:ty ; )*) => {
        tauri::generate_handler![ $( $crate::commands::$name ),* ]
    };
}

/// `invoke_handler` của app (cũng dùng trong test với `mock_builder`).
#[macro_export]
macro_rules! app_handler {
    () => {
        $crate::command_list!(__command_handler)
    };
}

command_list!(__command_table);

/// Chạy `f` ngoài luồng async của Tauri (SQLite, đĩa, CPU) rồi trả kết quả.
pub async fn blocking<T, F>(state: &State<'_, Arc<AppState>>, f: F) -> Result<T, CmdError>
where
    T: Send + 'static,
    F: FnOnce(&Arc<AppState>) -> Result<T, CmdError> + Send + 'static,
{
    let st = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || f(&st))
        .await
        .map_err(|e| CmdError::internal(format!("tác vụ nền lỗi: {e}")))?
}

/// Lưu PNG do UI vẽ: Rust mở hộp thoại lưu (lọc .png, ép đuôi), JS không cần quyền dialog/fs.
#[tauri::command]
pub async fn save_png<R: tauri::Runtime>(app: tauri::AppHandle<R>, suggested_name: String, png_base64: String) -> Result<Option<String>, CmdError> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::export::save_png_with(&suggested_name, &png_base64, &|name| {
            app.dialog()
                .file()
                .add_filter("PNG", &["png"])
                .set_file_name(name)
                .blocking_save_file()
                .and_then(|p| p.into_path().ok())
        })
    })
    .await
    .map_err(|e| CmdError::internal(format!("tác vụ nền lỗi: {e}")))?
}

#[tauri::command]
pub fn startup_status(state: State<'_, Arc<AppState>>) -> Result<BootStatusDto, CmdError> {
    Ok(boot::snapshot(&state))
}

#[tauri::command]
pub async fn boot_subscribe(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<BootEvent>,
) -> Result<(), CmdError> {
    // đăng ký TRƯỚC rồi mới gửi pha hiện tại (có thể trùng, không mất `Ready` rơi giữa hai bước)
    let ch = on_event.clone();
    state
        .hubs
        .boot
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    let _ = ch.send(BootEvent::Phase {
        phase: boot::snapshot(&state).phase,
    });
    Ok(())
}

#[tauri::command]
pub async fn onboarding_state(state: State<'_, Arc<AppState>>) -> Result<OnboardingDto, CmdError> {
    blocking(&state, |s| onboarding::onboarding_state(s)).await
}

#[tauri::command]
pub async fn set_onboarding_step(
    state: State<'_, Arc<AppState>>,
    step: Option<String>,
) -> Result<(), CmdError> {
    blocking(&state, move |s| onboarding::set_onboarding_step(s, step)).await
}

#[tauri::command]
pub async fn legacy_import(state: State<'_, Arc<AppState>>) -> Result<LegacyStateDto, CmdError> {
    blocking(&state, boot::legacy_import).await
}

#[tauri::command]
pub async fn legacy_retry(state: State<'_, Arc<AppState>>) -> Result<LegacyStateDto, CmdError> {
    blocking(&state, boot::legacy_import).await
}

macro_rules! cmd {
    ($(#[$m:meta])* $name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty => |$s:ident| $body:expr) => {
        $(#[$m])*
        #[tauri::command]
        pub async fn $name(state: State<'_, Arc<AppState>> $(, $arg: $ty)*) -> Result<$ret, CmdError> {
            blocking(&state, move |$s| $body).await
        }
    };
}

cmd!(query_faces(q: FaceQueryDto) -> FacePageDto => |s| library::query_faces(s, &q));
cmd!(family_groups(q: FaceQueryDto) -> Vec<FamilyGroupDto> => |s| library::family_groups(s, &q));
cmd!(get_family_styles(family: String) -> Vec<FamilyStyleDto> => |s| library::get_family_styles(s, &family));
cmd!(set_favorite(face: FaceKeyDto, on: bool) -> () => |s| library::set_favorite(s, &face, on));
cmd!(list_collections() -> Vec<CollectionDto> => |s| library::list_collections(s));
cmd!(create_collection(name: String, color: Option<String>) -> CollectionDto => |s| library::create_collection(s, &name, color.as_deref()));
cmd!(rename_collection(id: i64, name: String) -> () => |s| library::rename_collection(s, id, &name));
cmd!(delete_collection(id: i64) -> () => |s| library::delete_collection(s, id));
cmd!(add_to_collection(id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::add_to_collection(s, id, &faces));
cmd!(remove_from_collection(id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::remove_from_collection(s, id, &faces));
cmd!(list_tags() -> Vec<TagDto> => |s| library::list_tags(s));
cmd!(system_tag_catalog() -> Vec<SystemTagGroupDto> => |_s| Ok(library::system_tag_catalog()));
cmd!(tags_of(face: FaceKeyDto) -> Vec<TagDto> => |s| library::tags_of(s, &face));
cmd!(create_tag(name: String) -> TagDto => |s| library::create_tag(s, &name));
cmd!(rename_tag(id: i64, name: String) -> TagDto => |s| library::rename_tag(s, id, &name));
cmd!(delete_tag(id: i64) -> u32 => |s| library::delete_tag(s, id));
cmd!(tag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::tag_faces(s, tag_id, &faces));
cmd!(untag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::untag_faces(s, tag_id, &faces));
cmd!(list_problems() -> Vec<ProblemDto> => |s| library::list_problems(s));
cmd!(retry_problems() -> u32 => |s| library::retry_problems(s));
cmd!(reveal_in_finder(face: FaceKeyDto) -> () => |s| system::reveal_in_finder(s, &face, &|p| {
    tauri_plugin_opener::reveal_item_in_dir(p).map_err(|e| e.to_string())
}));
cmd!(backup_user_db() -> String => |s| system::backup_user_db(s));
cmd!(auto_status() -> AutoStatusDto => |s| auto::status(s));
cmd!(auto_set_enabled(on: bool) -> AutoStatusDto => |s| auto::set_enabled(s, on));
cmd!(auto_install(target: AutoTargetDto) -> AutoStatusDto => |s| auto::install(s, target));
cmd!(auto_remove(target: AutoTargetDto) -> AutoStatusDto => |s| auto::remove(s, target));
cmd!(auto_log(limit: Option<u32>) -> Vec<AutoLogDto> => |s| auto::log(s, limit));
cmd!(auto_forget_key() -> AutoStatusDto => |s| auto::forget_key(s));
cmd!(auto_probe() -> AutoStatusDto => |s| auto::probe(s));
cmd!(disk_free(target: String) -> u64 => |s| system::disk_free(s, &target));
cmd!(get_ui_state() -> UiStateDto => |s| system::get_ui_state(s));
cmd!(set_ui_state(state: UiStateDto) -> () => |s| system::set_ui_state(s, &state));
cmd!(get_setting(key: String) -> Option<String> => |s| system::get_setting(s, &key));
cmd!(set_setting(key: String, value: String) -> () => |s| system::set_setting(s, &key, &value));
cmd!(get_face(face: FaceKeyDto, lang: LangDto) -> FaceDetailDto => |s| analysis::get_face(s, &face, lang));
cmd!(similar(face: FaceKeyDto, opts: SimilarOptsDto) -> Vec<SimilarHitDto> => |s| analysis::similar(s, &face, &opts));
cmd!(similar_families(face: FaceKeyDto, opts: SimilarOptsDto, q: FaceQueryDto) -> SimilarFamilyPageDto => |s| analysis::similar_families(s, &face, &opts, &q));
cmd!(axis_stats(q: FaceQueryDto) -> Vec<AxisStatsDto> => |s| analysis::axis_stats(s, &q));
cmd!(axis_histogram(q: FaceQueryDto, bins: u32) -> Vec<AxisHistogramDto> => |s| analysis::axis_histograms(s, &q, bins));
cmd!(class_facets(q: FaceQueryDto) -> Vec<ClassFacetDto> => |s| analysis::class_facets(s, &q));
cmd!(pair_candidates(face: FaceKeyDto, opts: PairOptsDto) -> Vec<PairCandidateDto> => |s| analysis::pair_candidates(s, &face, &opts));
cmd!(role_fit(face: FaceKeyDto, role: RoleRefDto) -> RoleFitDto => |s| suggest::role_fit(s, &face, &role));
cmd!(role_fit_faces(role: RoleRefDto, opts: RoleFitListOptsDto) -> RoleFitPageDto => |s| suggest::role_fit_faces(s, &role, &opts));
cmd!(suggest_partners(anchor: FaceKeyDto, req: SuggestPartnersReqDto) -> PartnerPageDto => |s| suggest_pair::suggest_partners(s, &anchor, &req));
cmd!(desk_suggest(req: DeskSuggestReqDto) -> DeskPageDto => |s| suggest_pair::desk_suggest(s, &req));
cmd!(suggest_combos(req: SuggestCombosReqDto) -> Vec<ComboSuggestionDto> => |s| suggest_pair::suggest_combos(s, &req));
cmd!(record_view(face: FaceKeyDto) -> () => |s| suggest::record_view(s, &face));
cmd!(recent_views(limit: u32) -> Vec<RecentViewDto> => |s| suggest::recent_views(s, limit));
cmd!(clear_recent_views() -> u32 => |s| suggest::clear_recent_views(s));
cmd!(assess_pair(a: FaceKeyDto, b: FaceKeyDto, opts: PairOptsDto) -> PairCandidateDto => |s| analysis::assess_pair(s, &a, &b, &opts));
cmd!(reveal_location(path: String) -> () => |s| system::reveal_location(s, &path, &|p| {
    tauri_plugin_opener::reveal_item_in_dir(p).map_err(|e| e.to_string())
}));
cmd!(language_list(q: FaceQueryDto) -> LangListDto => |s| langs::language_list(s, &q));
cmd!(lang_coverage(face_id: i64, code: String) -> LangCoverageDto => |s| langs::lang_coverage(s, face_id, &code));
cmd!(duplicate_groups(q: FaceQueryDto) -> Vec<DuplicateGroupDto> => |s| duplicates::duplicate_groups(s, &q));
cmd!(duplicate_summary(q: FaceQueryDto) -> DuplicateSummaryDto => |s| duplicates::duplicate_summary(s, &q));
cmd!(resolve_preview(scope: ResolveScopeDto, q: FaceQueryDto) -> ResolvePlanDto => |s| dup_resolve::resolve_preview(s, &scope, &q));
cmd!(resolve_apply(req: ResolveRequestDto) -> ResolveResultDto => |s| dup_resolve::resolve_apply(s, &req, &crate::trash::trash_item, dup_resolve::now_ms()));
cmd!(resolve_undo(ids: Vec<i64>) -> ResolveResultDto => |s| dup_resolve::resolve_undo(s, &ids, dup_resolve::now_ms()));
cmd!(resolve_list(include_undone: bool) -> Vec<ResolutionDto> => |s| dup_resolve::resolve_list(s, include_undone));
cmd!(duplicate_ignore(kind: DuplicateKindDto, key: String) -> () => |s| duplicates::duplicate_ignore(s, kind, &key, dup_resolve::now_ms()));
cmd!(duplicate_unignore(kind: DuplicateKindDto, key: String) -> () => |s| duplicates::duplicate_unignore(s, kind, &key));
cmd!(duplicate_ignored() -> Vec<IgnoredGroupDto> => |s| duplicates::duplicate_ignored(s));
cmd!(source_priority() -> Vec<i64> => |s| dup_resolve::source_priority(s));
cmd!(set_source_priority(ids: Vec<i64>) -> Vec<i64> => |s| dup_resolve::set_source_priority(s, &ids));
cmd!(compare_faces(faces: Vec<FaceKeyDto>, opts: CompareOptsDto) -> CompareDto => |s| compare::compare_faces(s, &faces, &opts));
cmd!(pair_these(faces: Vec<FaceKeyDto>, opts: PairTheseOptsDto) -> Vec<PairComboDto> => |s| compare::pair_these(s, &faces, &opts));
cmd!(save_pair(pair: NewPairDto) -> i64 => |s| pairs::save_pair(s, &pair));
cmd!(list_pairs(collection_id: Option<i64>) -> Vec<SavedPairDto> => |s| pairs::list_pairs(s, collection_id));
cmd!(delete_pair(id: i64) -> () => |s| pairs::delete_pair(s, id));
cmd!(set_classification_override(over: ClassificationOverrideDto) -> () => |s| analysis::set_classification_override(s, &over));
cmd!(clear_classification_override(face: FaceKeyDto) -> () => |s| analysis::clear_classification_override(s, &face));
cmd!(describe(text: String) -> DescribeDto => |s| Ok(describe::describe(s, &text)));
cmd!(create_case(name: Option<String>, brief: Option<BriefDto>, settings: Option<CaseSettingsDto>) -> CaseDto => |s| cases::create_case(s, name.as_deref(), brief.as_ref(), settings.as_ref()));
cmd!(list_cases(include_deleted: bool) -> Vec<CaseSummaryDto> => |s| cases::list_cases(s, include_deleted));
cmd!(open_case(id: i64) -> CaseDto => |s| cases::open_case(s, id));
cmd!(get_case(id: i64) -> CaseDto => |s| cases::get_case(s, id));
cmd!(update_case(id: i64, patch: CasePatchDto) -> CaseDto => |s| cases::update_case(s, id, &patch));
cmd!(delete_case(id: i64) -> () => |s| cases::delete_case(s, id));
cmd!(restore_case(id: i64) -> CaseDto => |s| cases::restore_case(s, id));
cmd!(add_to_case(case_id: Option<i64>, faces: Vec<FaceKeyDto>, origin: Option<CaseOriginDto>) -> AddToCaseDto => |s| cases::add_to_case(s, case_id, &faces, origin));
cmd!(remove_from_case(case_id: i64, faces: Vec<FaceKeyDto>) -> u32 => |s| cases::remove_from_case(s, case_id, &faces));
cmd!(create_combo(case_id: i64, name: Option<String>, from_combo: Option<i64>) -> ComboDto => |s| cases::create_combo(s, case_id, name.as_deref(), from_combo));
cmd!(update_combo(id: i64, patch: ComboPatchDto) -> ComboDto => |s| cases::update_combo(s, id, &patch));
cmd!(delete_combo(id: i64) -> () => |s| cases::delete_combo(s, id));
cmd!(reorder_combos(case_id: i64, ids: Vec<i64>) -> () => |s| cases::reorder_combos(s, case_id, &ids));
cmd!(set_combo_slot(combo_id: i64, role: ComboRoleDto, face: Option<FaceKeyDto>, note: Option<String>, style: Option<serde_json::Value>) -> ComboDto => |s| cases::set_combo_slot(s, combo_id, role, face.as_ref(), note.as_deref(), style.as_ref()));
cmd!(add_combo_role(combo_id: i64, name: String) -> ComboDto => |s| cases::add_combo_role(s, combo_id, &name));
cmd!(remove_combo_role(combo_id: i64, role: ComboRoleDto) -> ComboDto => |s| cases::remove_combo_role(s, combo_id, &role));
cmd!(rename_combo_role(combo_id: i64, role: ComboRoleDto, new_name: String) -> ComboDto => |s| cases::rename_combo_role(s, combo_id, &role, &new_name));
cmd!(reorder_combo_roles(combo_id: i64, roles: Vec<ComboRoleDto>) -> ComboDto => |s| cases::reorder_combo_roles(s, combo_id, &roles));
cmd!(case_compare(case_id: i64, combo_ids: Vec<i64>, lang: LangDto) -> CaseCompareDto => |s| cases::case_compare(s, case_id, &combo_ids, lang));
cmd!(brief_starter(brief: BriefDto, case_id: Option<i64>) -> BriefStarterDto => |s| brief::brief_starter(s, &brief, case_id));
cmd!(open_case_from_peek(peek: PeekOpenDto) -> PeekOpenResultDto => |s| cases::open_case_from_peek(s, &peek));
cmd!(create_combos(case_id: i64, combos: Vec<NewComboDto>) -> Vec<ComboDto> => |s| cases::create_combos(s, case_id, &combos));
cmd!(get_settings() -> SettingsDto => |s| settings::get_settings(s));
cmd!(set_settings(patch: SettingsPatchDto) -> SettingsDto => |s| settings::set_settings(s, patch));
cmd!(model_status() -> ModelStatusDto => |s| Ok(model::model_status(s)));
cmd!(model_cancel() -> () => |s| {
    model::cancel_download(s);
    Ok(())
});
cmd!(model_remove() -> () => |s| model::remove(s));
cmd!(embed_status() -> EmbedStatusDto => |s| embed::embed_status(s));
cmd!(embed_pause() -> () => |s| {
    s.embed.pause();
    Ok(())
});
cmd!(embed_resume() -> () => |s| {
    s.embed.resume();
    Ok(())
});
cmd!(embed_failures() -> Vec<EmbedFailureDto> => |s| embed::embed_failures(s));
cmd!(queue_pause_all() -> () => |s| {
    embed::pause_all(s);
    Ok(())
});
cmd!(queue_resume_all() -> () => |s| {
    embed::resume_all(s);
    Ok(())
});

#[tauri::command]
pub async fn model_download(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ModelEvent>,
) -> Result<(), CmdError> {
    state
        .hubs
        .model
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    blocking(&state, model::start_download).await
}

#[tauri::command]
pub async fn model_redownload(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ModelEvent>,
) -> Result<(), CmdError> {
    state
        .hubs
        .model
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    blocking(&state, model::redownload).await
}

#[tauri::command]
pub async fn embed_subscribe(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<EmbedEvent>,
) -> Result<(), CmdError> {
    let ch = on_event.clone();
    state
        .hubs
        .embed
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    if let Ok(status) = blocking(&state, |s| embed::embed_status(s)).await {
        let _ = ch.send(EmbedEvent::Status { status });
    }
    Ok(())
}

cmd!(activation_state() -> Vec<ActivationStateDto> => |s| activation::state_list(s));
cmd!(activation_preflight(faces: Vec<FaceKeyDto>) -> ActivationReportDto => |s| activation::preflight(s, &faces));
cmd!(activation_activate(faces: Vec<FaceKeyDto>, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::activate(s, &faces, scope, proceed, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_list_replaced() -> Vec<ReplacedEntryDto> => |s| activation::list_replaced(s));
cmd!(activation_replace(faces: Vec<FaceKeyDto>, scope: Option<ScopeDto>, confirmed: Vec<ConfirmedConflictDto>, on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::replace(s, &faces, scope, &confirmed, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_restore_replaced(id: i64) -> RestoreOutcomeDto => |s| activation::restore_replaced(s, id));
cmd!(activation_activate_case(case_id: i64, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> CaseActivationDto => |s| {
    activation::activate_case(s, case_id, scope, proceed, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_activate_collection(collection_id: i64, scope: Option<ScopeDto>, proceed: bool, on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::activate_collection(s, collection_id, scope, proceed, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_deactivate(faces: Vec<FaceKeyDto>, force: bool, on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::deactivate(s, &faces, force, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_deactivate_collection(collection_id: i64, on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::deactivate_collection(s, collection_id, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_deactivate_all(on_progress: Channel<ProgressDto>) -> ActivationReportDto => |s| {
    activation::deactivate_all(s, &mut |p| {
        let _ = on_progress.send(p);
    })
});
cmd!(activation_set_keep(on: bool) -> () => |s| activation::set_keep(s, on));
cmd!(activation_availability(faces: Vec<FaceKeyDto>) -> Vec<AvailabilityDto> => |s| activation::availability(s, &faces));
cmd!(activation_set_scope(faces: Vec<FaceKeyDto>, scope: ScopeDto) -> ActivationReportDto => |s| activation::set_scope(s, &faces, scope));

#[tauri::command]
pub async fn activation_subscribe(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ActivationEvent>,
) -> Result<(), CmdError> {
    state
        .hubs
        .activation
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    Ok(())
}

cmd!(preview_bump_generation() -> u64 => |s| preview::bump_generation(s));
cmd!(preview_prewarm(items: Vec<PreviewItemDto>) -> () => |s| preview::prewarm(s, &items));
cmd!(missing_chars(face: FaceKeyDto, chars: Vec<String>) -> Vec<String> => |s| preview::missing_chars(s, &face, &chars));
cmd!(preview_cache_info() -> PreviewCacheInfoDto => |s| preview::cache_info(s));
cmd!(clear_preview_cache() -> () => |s| preview::clear_cache(s));

#[tauri::command]
pub fn preview_info(state: State<'_, Arc<AppState>>) -> Result<PreviewInfoDto, CmdError> {
    preview::preview_info(&state)
}

cmd!(detect_font_managers() -> Vec<FontManagerDto> => |_s| Ok(system::detect_font_managers()));

#[tauri::command]
pub async fn list_sources(state: State<'_, Arc<AppState>>) -> Result<Vec<SourceDto>, CmdError> {
    blocking(&state, |s| sources::list_sources(s)).await
}

#[tauri::command]
pub async fn add_source(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<SourceDto, CmdError> {
    blocking(&state, move |s| sources::add_source(s, &path)).await
}

#[tauri::command]
pub async fn remove_source(
    state: State<'_, Arc<AppState>>,
    source_id: i64,
) -> Result<RemoveSourceReportDto, CmdError> {
    blocking(&state, move |s| sources::remove_source(s, source_id)).await
}

#[tauri::command]
pub async fn remove_source_preview(
    state: State<'_, Arc<AppState>>,
    source_id: i64,
) -> Result<RemoveSourceReportDto, CmdError> {
    blocking(&state, move |s| sources::remove_source_preview(s, source_id)).await
}

#[tauri::command]
pub async fn rescan_sources(
    state: State<'_, Arc<AppState>>,
    source_ids: Option<Vec<i64>>,
) -> Result<(), CmdError> {
    blocking(&state, move |s| sources::rescan_sources(s, source_ids)).await
}

#[tauri::command]
pub async fn scan_pause(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.pause(&state);
    Ok(())
}

#[tauri::command]
pub async fn scan_resume(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.resume(&state);
    Ok(())
}

#[tauri::command]
pub async fn scan_cancel(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.cancel_current();
    Ok(())
}

#[tauri::command]
pub fn scan_status(state: State<'_, Arc<AppState>>) -> Result<ScanStatusDto, CmdError> {
    Ok(state.scans.status())
}

#[tauri::command]
pub async fn scan_subscribe(
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ScanEvent>,
) -> Result<(), CmdError> {
    state
        .hubs
        .scan
        .subscribe_while(Box::new(move |e| on_event.send(e).is_ok()));
    Ok(())
}

#[tauri::command]
pub fn activation_caps(state: State<'_, Arc<AppState>>) -> Result<ActivationCapsDto, CmdError> {
    Ok(activation::caps(&state))
}

/// Kiểm kỷ luật async trên `table` và mã nguồn `src` của commands.rs; trả danh sách vi phạm.
pub fn async_violations(table: &[CommandSig], src: &str) -> Vec<String> {
    let mut v = Vec::new();
    for c in table {
        let sync_def = format!("pub fn {}(", c.name);
        let async_def = format!("pub async fn {}(", c.name);
        if !c.is_async && !SYNC_OK.contains(&c.name) {
            v.push(format!(
                "{} khai báo sync nhưng không nằm trong SYNC_OK",
                c.name
            ));
        }
        // `cmd!(tên(` sinh `pub async fn tên(`
        let via_macro = format!("cmd!({}(", c.name);
        // lệnh generic theo Runtime (cần AppHandle<R> cho dialog): `pub async fn tên<R: …>(`
        let async_generic = format!("pub async fn {}<", c.name);
        if c.is_async && !src.contains(&async_def) && !src.contains(&async_generic) && !src.contains(&via_macro) {
            v.push(format!(
                "{} khai báo async trong bảng nhưng hàm không phải `pub async fn`",
                c.name
            ));
        }
        if !c.is_async && !src.contains(&sync_def) {
            v.push(format!(
                "{} khai báo sync trong bảng nhưng không thấy `pub fn`",
                c.name
            ));
        }
    }
    // mọi `#[tauri::command]` trần phải đi cùng hàm trong SYNC_OK
    let lines: Vec<&str> = src.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "#[tauri::command]"
            && let Some(next) = lines.get(i + 1)
            && let Some(rest) = next.trim().strip_prefix("pub fn ")
        {
            let name = rest.split('(').next().unwrap_or("");
            if !SYNC_OK.contains(&name) {
                v.push(format!("{name}: lệnh đồng bộ ngoài SYNC_OK"));
            }
        }
    }
    v
}
