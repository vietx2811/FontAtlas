// Cài đặt thật của Api: invoke + Channel của Tauri, kiểu từ UX/bindings (ts-rs). Tên lệnh và khoá đối số do TauriCommandMap kiểm lúc biên dịch.
import { COMPARE_MAX, COMPARE_MIN } from './compare';
import { fromWire } from './compareWire';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';
import type { CommandName, FaceQueryDto, SettingsPatchDto, TauriCommandMap } from '../bindings';
import { makeFontUrl, makePreviewUrl } from '../features/library-grid/api';
import { parseFacePage, type FaceQuery } from '../features/library-grid/types';
import type { Api } from './index';
import { CmdError, type SimilarQuery, type Unsubscribe } from './types';
import { t, getLocale, hasKey } from '../i18n';

/** Đọc Blob thành ArrayBuffer (jsdom và WebView cũ chưa có Blob.arrayBuffer: dùng FileReader). */
const blobBuffer = (b: Blob): Promise<ArrayBuffer> => (typeof b.arrayBuffer === 'function' ? b.arrayBuffer() : new Promise((res, rej) => { const r = new FileReader(); r.onload = () => res(r.result as ArrayBuffer); r.onerror = () => rej(r.error); r.readAsArrayBuffer(b); }));

/**
 * Backend viết thông báo lỗi bằng tiếng Việt (chi tiết kỹ thuật cho nhật ký). Giao diện tiếng Anh thay bằng câu theo `code`; tiếng Việt giữ nguyên câu gốc.
 */
export function localizeCmdMessage(code: string, message: string): string {
  if (getLocale() === 'vi') return message;
  const key = `err.${code}`;
  return hasKey(key) ? t(key) : t('err.generic', { code });
}

/** Lệnh Tauri reject bằng object `{code, message, detail}` (CmdError của 6a); gói lại thành lớp để `instanceof` được. */
export function toCmdError(e: unknown): CmdError {
  if (e instanceof CmdError) return e;
  if (typeof e === 'object' && e !== null && 'code' in e && 'message' in e) {
    const x = e as { code: string; message: string; detail?: unknown };
    return new CmdError(x.code, localizeCmdMessage(x.code, x.message), x.detail ?? undefined);
  }
  return new CmdError('internal', typeof e === 'string' ? e : (e as Error)?.message ?? t('api.unknownError'));
}

type Args<K extends CommandName> = TauriCommandMap[K]['args'];
type Result<K extends CommandName> = TauriCommandMap[K]['result'];

async function call<K extends CommandName>(cmd: K, ...args: Args<K> extends Record<string, never> ? [] : [Args<K>]): Promise<Result<K>> {
  try {
    return await invoke<Result<K>>(cmd, (args[0] ?? {}) as Record<string, unknown>);
  } catch (e) {
    throw toCmdError(e);
  }
}

/** Channel nhận sự kiện; huỷ đăng ký bằng cờ (Channel của Tauri không có hàm huỷ). */
function channel<T>(onEvent: ((e: T) => void) | undefined): { ch: Channel<T>; off: Unsubscribe } {
  const ch = new Channel<T>();
  let on = true;
  ch.onmessage = e => { if (on) onEvent?.(e); };
  return { ch, off: () => { on = false; } };
}
async function subscribe<T>(cmd: 'boot_subscribe' | 'scan_subscribe' | 'embed_subscribe' | 'activation_subscribe', onEvent: (e: T) => void): Promise<Unsubscribe> {
  const { ch, off } = channel<T>(onEvent);
  await call(cmd, { onEvent: ch } as never);
  return off;
}

export function toFaceQueryDto(q: FaceQuery): FaceQueryDto {
  return {
    text: q.text?.trim() ? q.text : null, vi: q.vi ?? null, license: q.license ?? [], variable: q.variable ?? null, italic: q.italic ?? null,
    sourceIds: q.sourceIds ?? [], favoritesOnly: q.favoritesOnly ?? false, collectionId: q.collectionId ?? null, tagIds: q.tagIds ?? [],
    includeMissing: q.includeMissing ?? false, sort: q.sort ?? 'auto', descending: q.descending ?? false, offset: q.offset ?? 0, limit: q.limit ?? 100,
    axisFilters: [], hard: [], rawAxisRanges: q.rawAxisRanges ?? [], faceKeys: q.faceKeys ?? null,
    classes: q.classes ?? [], subclasses: q.subclasses ?? [], traits: q.traits ?? [],
    includeSecondary: q.includeSecondary ?? false, confidentOnly: q.confidentOnly ?? false, minConfidence: q.minConfidence ?? null,
    langs: q.langs ?? [], langMode: q.langMode ?? 'all', badgeLang: q.badgeLang ?? null,
    weightClass: q.weightClass ?? null, widthClass: q.widthClass ?? null, varAxisMatch: q.varAxisMatch ?? { wght: true, wdth: true },
    groupByFamily: q.groupByFamily ?? false,
  };
}
const similarOpts = (o: SimilarQuery = {}) => ({
  weights: o.weights ?? { proportion: 1, weight: 1, structure: 1, detail: 1 }, useImage: o.useImage ?? true, k: o.k ?? 12, viOnly: o.viOnly ?? false,
  sourceIds: o.sourceIds ?? [], weightRange: o.weightRange ?? null,
});
const settingsPatch = (p: Partial<Record<keyof SettingsPatchDto, unknown>>): SettingsPatchDto => ({
  embedEnabled: (p.embedEnabled as boolean) ?? null, licenseNoticeAck: (p.licenseNoticeAck as boolean) ?? null, uiLang: (p.uiLang as string) ?? null,
  sampleText: (p.sampleText as string) ?? null, projectViDefault: (p.projectViDefault as boolean) ?? null, keepActiveOnExit: (p.keepActiveOnExit as boolean) ?? null,
  rescanOnLaunch: (p.rescanOnLaunch as boolean) ?? null, cloudPreviewDownload: (p.cloudPreviewDownload as boolean) ?? null,
});

export function createTauriApi(): Api {
  const previewUrl = makePreviewUrl(name => convertFileSrc(name, 'fontatlas-preview'));
  const fontUrl = makeFontUrl(name => convertFileSrc(name, 'fontatlas-font'));
  const none = async (p: Promise<unknown>): Promise<void> => { await p; };
  return {
    // Xem trước và lưới
    queryFaces: async q => parseFacePage(await call('query_faces', { q: toFaceQueryDto(q) })),
    familyGroups: (q = {}) => call('family_groups', { q: toFaceQueryDto(q) }),
    previewInfo: () => call('preview_info'),
    bumpGeneration: () => call('preview_bump_generation'),
    previewUrl, fontUrl,
    // Khởi động, onboarding
    startupStatus: () => call('startup_status'),
    bootSubscribe: onEvent => subscribe('boot_subscribe', onEvent),
    onboardingState: () => call('onboarding_state'),
    setOnboardingStep: step => none(call('set_onboarding_step', { step })),
    legacyImport: () => call('legacy_import'),
    legacyRetry: () => call('legacy_retry'),
    // Nguồn, quét
    listSources: () => call('list_sources'),
    addSource: path => call('add_source', { path }),
    removeSource: sourceId => none(call('remove_source', { sourceId })),
    rescanSources: sourceIds => none(call('rescan_sources', { sourceIds: sourceIds ?? null })),
    scanPause: () => none(call('scan_pause')), scanResume: () => none(call('scan_resume')), scanCancel: () => none(call('scan_cancel')),
    scanStatus: () => call('scan_status'),
    scanSubscribe: onEvent => subscribe('scan_subscribe', onEvent),
    // Thư viện
    getFamilyStyles: family => call('get_family_styles', { family }),
    setFavorite: (face, on) => none(call('set_favorite', { face, on })),
    listCollections: () => call('list_collections'),
    createCollection: (name, color) => call('create_collection', { name, color }),
    renameCollection: (id, name) => none(call('rename_collection', { id, name })),
    deleteCollection: id => none(call('delete_collection', { id })),
    addToCollection: (id, faces) => none(call('add_to_collection', { id, faces })),
    removeFromCollection: (id, faces) => none(call('remove_from_collection', { id, faces })),
    listTags: () => call('list_tags'),
    systemTagCatalog: () => call('system_tag_catalog'),
    tagsOf: face => call('tags_of', { face }),
    createTag: name => call('create_tag', { name }),
    renameTag: (id, name) => call('rename_tag', { id, name }),
    deleteTag: id => call('delete_tag', { id }),
    tagFaces: (tagId, faces) => none(call('tag_faces', { tagId, faces })),
    untagFaces: (tagId, faces) => none(call('untag_faces', { tagId, faces })),
    revealInFinder: face => none(call('reveal_in_finder', { face })),
    getUiState: () => call('get_ui_state'),
    setUiState: state => none(call('set_ui_state', { state })),
    // Phân tích
    getFace: (face, lang) => call('get_face', { face, lang }),
    similar: (face, opts) => call('similar', { face, opts: similarOpts(opts) }),
    similarFamilies: (face, opts, q = {}) => call('similar_families', { face, opts: similarOpts(opts), q: toFaceQueryDto(q) }),
    languageList: (q = {}) => call('language_list', { q: toFaceQueryDto({ ...q, langs: [] }) }),
    langCoverage: (faceId, code) => call('lang_coverage', { faceId, code }),
    axisStats: (q = {}) => call('axis_stats', { q: toFaceQueryDto(q) }),
    createCase: (name, brief, settings) => call('create_case', { name: name ?? null, brief: brief ?? null, settings: settings ?? null }),
    listCases: (includeDeleted = false) => call('list_cases', { includeDeleted }),
    openCase: id => call('open_case', { id }),
    getCase: id => call('get_case', { id }),
    updateCase: (id, patch) => call('update_case', { id, patch: { name: null, brief: null, clearBrief: false, settings: null, viewState: null, ...patch } }),
    deleteCase: id => none(call('delete_case', { id })),
    restoreCase: id => call('restore_case', { id }),
    addToCase: (caseId, faces, origin) => call('add_to_case', { caseId, faces, origin: origin ?? null }),
    removeFromCase: (caseId, faces) => call('remove_from_case', { caseId, faces }),
    createCombo: (caseId, name, fromCombo) => call('create_combo', { caseId, name: name ?? null, fromCombo: fromCombo ?? null }),
    updateCombo: (id, patch) => call('update_combo', { id, patch: { name: null, isMain: null, note: null, clearNote: false, ...patch } }),
    deleteCombo: id => none(call('delete_combo', { id })),
    reorderCombos: (caseId, ids) => none(call('reorder_combos', { caseId, ids })),
    setComboSlot: (comboId, role, face, note, style) => call('set_combo_slot', { comboId, role, face, note: note ?? null, style: style ?? null }),
    createCombos: (caseId, combos) => call('create_combos', { caseId, combos }),
    caseCompare: (caseId, comboIds, lang = getLocale()) => call('case_compare', { caseId, comboIds, lang }),
    briefStarter: (brief, caseId) => call('brief_starter', { brief, caseId: caseId ?? null }),
    openCaseFromPeek: peek => call('open_case_from_peek', { peek: { replace: false, ...peek } }),
    axisHistogram: (q = {}, bins = 0) => call('axis_histogram', { q: toFaceQueryDto(q), bins }),
    async compareFaces(faces) {
      if (faces.length < COMPARE_MIN || faces.length > COMPARE_MAX) throw new Error(`compare needs ${COMPARE_MIN}–${COMPARE_MAX} faces, got ${faces.length}`);
      return fromWire(await call('compare_faces', { faces, opts: { lang: getLocale() } }));
    },
    async pairThese(faces, o = {}) {
      if (faces.length < 2 || faces.length > 4) throw new Error(`pair these needs 2–4 faces, got ${faces.length}`);
      return call('pair_these', { faces, opts: { projectVi: o.projectVi ?? true, lang: getLocale(), maxCombos: o.maxCombos ?? 3 } });
    },
    async savePng(png, suggestedName) {
      // Backend mở hộp thoại Lưu (lọc .png), kiểm PNG magic và ≤ 32 MB, rồi ghi file; trả đường dẫn hoặc null nếu huỷ.
      const bytes = new Uint8Array(await blobBuffer(png));
      let bin = ''; for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      return call('save_png', { suggestedName, pngBase64: btoa(bin) });
    },
    revealLocation: path => none(call('reveal_location', { path })),
    missingChars: (face, chars) => call('missing_chars', { face, chars }),
    duplicateSummary: (q = {}) => call('duplicate_summary', { q: toFaceQueryDto(q) }),
    duplicateGroups: (q = {}) => call('duplicate_groups', { q: toFaceQueryDto(q) }),
    resolvePreview: (scope, q = {}) => call('resolve_preview', { scope, q: toFaceQueryDto(q) }),
    resolveApply: req => call('resolve_apply', { req }),
    resolveUndo: ids => call('resolve_undo', { ids }),
    resolveList: includeUndone => call('resolve_list', { includeUndone }),
    duplicateIgnore: (kind, key) => none(call('duplicate_ignore', { kind, key })),
    duplicateUnignore: (kind, key) => none(call('duplicate_unignore', { kind, key })),
    duplicateIgnored: () => call('duplicate_ignored'),
    sourcePriority: () => call('source_priority'),
    setSourcePriority: ids => call('set_source_priority', { ids }),
    classFacets: (q = {}) => call('class_facets', { q: toFaceQueryDto(q) }),
    setClassificationOverride: over => none(call('set_classification_override', { over })),
    clearClassificationOverride: face => none(call('clear_classification_override', { face })),
    pairCandidates: (face, opts) => call('pair_candidates', { face, opts }),
    // Dữ liệu và hệ thống
    backupUserDb: () => call('backup_user_db'),
    previewCacheInfo: () => call('preview_cache_info'),
    clearPreviewCache: () => none(call('clear_preview_cache')),
    listProblems: () => call('list_problems'),
    retryProblems: () => call('retry_problems'),
    // Cài đặt
    getSetting: key => call('get_setting', { key }),
    setSetting: (key, value) => none(call('set_setting', { key, value })),
    getSettings: () => call('get_settings'),
    setSettings: patch => call('set_settings', { patch: settingsPatch(patch) }),
    // Model, embed
    autoStatus: () => call('auto_status'),
    autoSetEnabled: on => call('auto_set_enabled', { on }),
    autoInstall: target => call('auto_install', { target }),
    autoRemove: target => call('auto_remove', { target }),
    autoForgetKey: () => call('auto_forget_key'),
    autoProbe: () => call('auto_probe'),
    autoLog: limit => call('auto_log', { limit: limit ?? null }),
    modelStatus: () => call('model_status'),
    modelDownload: onEvent => none(call('model_download', { onEvent: channel(onEvent).ch })),
    modelCancel: () => none(call('model_cancel')),
    modelRedownload: onEvent => none(call('model_redownload', { onEvent: channel(onEvent).ch })),
    modelRemove: () => none(call('model_remove')),
    embedStatus: () => call('embed_status'),
    embedPause: () => none(call('embed_pause')), embedResume: () => none(call('embed_resume')),
    embedSubscribe: onEvent => subscribe('embed_subscribe', onEvent),
    // Kích hoạt
    activationCaps: () => call('activation_caps'),
    activationState: () => call('activation_state'),
    activationPreflight: faces => call('activation_preflight', { faces }),
    activationActivate: (faces, o = {}) => call('activation_activate', { faces, scope: o.scope ?? null, proceed: o.proceed ?? false, onProgress: channel(o.onProgress).ch }),
    activationReplace: (faces, confirmed, o = {}) => call('activation_replace', { faces, scope: o.scope ?? null, confirmed, onProgress: channel(o.onProgress).ch }),
    activationListReplaced: () => call('activation_list_replaced'),
    activationRestoreReplaced: id => call('activation_restore_replaced', { id }),
    activationDeactivate: (faces, o = {}) => call('activation_deactivate', { faces, force: o.force ?? false, onProgress: channel(o.onProgress).ch }),
    activationActivateCollection: (collectionId, o = {}) => call('activation_activate_collection', { collectionId, scope: o.scope ?? null, proceed: o.proceed ?? false, onProgress: channel(o.onProgress).ch }),
    activationDeactivateCollection: (collectionId, o = {}) => call('activation_deactivate_collection', { collectionId, onProgress: channel(o.onProgress).ch }),
    activationDeactivateAll: (o = {}) => call('activation_deactivate_all', { onProgress: channel(o.onProgress).ch }),
    activationSetKeep: on => none(call('activation_set_keep', { on })),
    activationSetScope: (faces, scope) => call('activation_set_scope', { faces, scope }),
    activationAvailability: (faces = []) => call('activation_availability', { faces }),
    activationSubscribe: onEvent => subscribe('activation_subscribe', onEvent),
  };
}
