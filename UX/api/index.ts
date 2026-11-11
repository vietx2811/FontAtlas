// Mọi lệnh backend đi qua một interface; tên phương thức = tên lệnh 6a (camelCase).
// Cài đặt thật: api/tauri.ts (invoke + Channel). Cài đặt giả: api/mock.ts (dev không có Tauri, Vitest).
import type { CompareDto } from './compare';
import type { LibraryApi } from '../features/library-grid/api';
import type {
  ActivationEvent, ActivationReportDto, ActivationStateDto, AvailabilityDto, RestoreOutcomeDto, ConfirmedConflictDto, ReplacedEntryDto, BootEvent, BootStatusDto, CollectionDto, EmbedEvent, EmbedStatusDto, FaceDetailDto,
  AxisStatsDto, AxisHistogramDto, LangListDto, LangCoverageDto, BriefDto, BriefStarterDto, CaseCompareDto, PeekOpenDto, PeekOpenResultDto, CaseSettingsDto, CaseDto, CaseSummaryDto, CasePatchDto, CaseOriginDto, AddToCaseDto, ComboDto, ComboPatchDto, ComboRoleDto, NewComboDto, DuplicateGroupDto, DuplicateSummaryDto, DuplicateKindDto, IgnoredGroupDto, ResolutionDto, ResolvePlanDto, ResolveRequestDto, ResolveResultDto, ResolveScopeDto, PairComboDto, PairTheseOpts, ClassFacetDto, ClassificationOverrideDto, SimilarFamilyPageDto, FaceQuery, FaceKeyDto, FamilyStyleDto, LegacyStateDto, PreviewCacheInfoDto, ProblemDto, ModelEvent, ModelStatusDto, OnboardingDto, PairCandidateDto, PairOptsDto, ProgressDto, ScanEvent,
  ScanStatusDto, ScopeDto, SettingsDto, SimilarHitDto, SimilarQuery, SourceDto, SystemTagGroupDto, TagDto, UiStateDto, Unsubscribe,
} from './types';

export * from './types';

export type ActivateOptions = { scope?: ScopeDto | null; proceed?: boolean; onProgress?(p: ProgressDto): void };
export type DeactivateOptions = { force?: boolean; onProgress?(p: ProgressDto): void };

export interface Api extends LibraryApi {
  // Khởi động và onboarding
  startupStatus(): Promise<BootStatusDto>;
  bootSubscribe(onEvent: (e: BootEvent) => void): Promise<Unsubscribe>;
  onboardingState(): Promise<OnboardingDto>;
  setOnboardingStep(step: string | null): Promise<void>;
  legacyImport(): Promise<LegacyStateDto>;
  legacyRetry(): Promise<LegacyStateDto>;
  // Nguồn và quét
  listSources(): Promise<SourceDto[]>;
  /** Ném CmdError('nested_source', …, { existingId, existingPath }) khi nằm trong/chứa nguồn có sẵn. */
  addSource(path: string): Promise<SourceDto>;
  removeSource(sourceId: number): Promise<void>;
  rescanSources(sourceIds?: number[]): Promise<void>;
  scanPause(): Promise<void>;
  scanResume(): Promise<void>;
  scanCancel(): Promise<void>;
  scanStatus(): Promise<ScanStatusDto>;
  scanSubscribe(onEvent: (e: ScanEvent) => void): Promise<Unsubscribe>;
  // Thư viện
  getFamilyStyles(family: string): Promise<FamilyStyleDto[]>;
  setFavorite(face: FaceKeyDto, on: boolean): Promise<void>;
  listCollections(): Promise<CollectionDto[]>;
  createCollection(name: string, color: string | null): Promise<CollectionDto>;
  renameCollection(id: number, name: string): Promise<void>;
  deleteCollection(id: number): Promise<void>;
  addToCollection(id: number, faces: FaceKeyDto[]): Promise<void>;
  removeFromCollection(id: number, faces: FaceKeyDto[]): Promise<void>;
  listTags(): Promise<TagDto[]>;
  /** Danh mục tag hệ thống (style/mood/use): chỉ là tag người dùng gán, không đo máy. Tên lưu `namespace:key`. */
  systemTagCatalog(): Promise<SystemTagGroupDto[]>;
  tagsOf(face: FaceKeyDto): Promise<TagDto[]>;
  createTag(name: string): Promise<TagDto>;
  tagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  untagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  revealInFinder(face: FaceKeyDto): Promise<void>;
  /** Mở Finder đúng một đường dẫn đã lưu trong thư viện (nhóm cùng tệp có nhiều đường dẫn cho một face). */
  revealLocation(path: string): Promise<void>;
  /** Grapheme (đã tách ở giao diện) mà font của face không có glyph, theo cmap thật. */
  missingChars(face: FaceKeyDto, chars: string[]): Promise<string[]>;
  getUiState(): Promise<UiStateDto>;
  setUiState(state: UiStateDto): Promise<void>;
  // Phân tích
  getFace(face: FaceKeyDto, lang: 'vi' | 'en'): Promise<FaceDetailDto>;
  similar(face: FaceKeyDto, opts?: SimilarQuery): Promise<SimilarHitDto[]>;
  similarFamilies(face: FaceKeyDto, opts?: SimilarQuery, q?: FaceQuery): Promise<SimilarFamilyPageDto>;
  axisStats(q?: FaceQuery): Promise<AxisStatsDto[]>;
  // Font Desk (0.3.0): Case + Combo
  createCase(name?: string, brief?: BriefDto, settings?: CaseSettingsDto): Promise<CaseDto>;
  listCases(includeDeleted?: boolean): Promise<CaseSummaryDto[]>;
  /** Mở Case (đánh dấu vừa mở) và trả đủ nội dung. */
  openCase(id: number): Promise<CaseDto>;
  getCase(id: number): Promise<CaseDto>;
  updateCase(id: number, patch: Partial<CasePatchDto>): Promise<CaseDto>;
  /** Xoá mềm 30 ngày. */
  deleteCase(id: number): Promise<void>;
  restoreCase(id: number): Promise<CaseDto>;
  /** `caseId` null = Case đang mở gần nhất, chưa có thì tạo mới (`created`). */
  addToCase(caseId: number | null, faces: FaceKeyDto[], origin?: CaseOriginDto): Promise<AddToCaseDto>;
  removeFromCase(caseId: number, faces: FaceKeyDto[]): Promise<number>;
  createCombo(caseId: number, name?: string, fromCombo?: number): Promise<ComboDto>;
  updateCombo(id: number, patch: Partial<ComboPatchDto>): Promise<ComboDto>;
  deleteCombo(id: number): Promise<void>;
  reorderCombos(caseId: number, ids: number[]): Promise<void>;
  /** `face` null = bỏ font khỏi vai. */
  setComboSlot(comboId: number, role: ComboRoleDto, face: FaceKeyDto | null, note?: string, style?: unknown): Promise<ComboDto>;
  createCombos(caseId: number, combos: NewComboDto[]): Promise<ComboDto[]>;
  /** Compare Combo: cột = Combo, kèm quan hệ giữa giọng ① và các vai khác. `comboIds` rỗng = mọi Combo (≤ 8). */
  caseCompare(caseId: number, comboIds: number[], lang?: 'vi' | 'en'): Promise<CaseCompareDto>;
  /** Brief starter: chip → bộ lọc + ≤3 neo + ≤3 Combo khởi đầu. Thuần đọc; `notes` là mã để dịch (vd `langs_incomplete`). */
  briefStarter(brief: BriefDto, caseId?: number): Promise<BriefStarterDto>;
  /** Pair from peek (2d): font làm neo ở vai `as`; ứng viên chỉ trả lại làm `trial`, chưa đặt vào vai. Vai đã có font khác ⇒ không ghi, trả `occupied`; gửi lại với `replace:true` sau khi người dùng đồng ý. `settings` chỉ dùng khi Case được tạo ở lệnh này. */
  openCaseFromPeek(peek: Omit<PeekOpenDto, 'replace'> & { replace?: boolean }): Promise<PeekOpenResultDto>;
  /** Histogram 18 trục (16 số đo + weight_class + width_class), mỗi trục đếm trên tập lọc TRỪ chính trục đó. `bins` 0 = 24 (tối đa 64); trục class luôn 9 bin. */
  axisHistogram(q?: FaceQuery, bins?: number): Promise<AxisHistogramDto[]>;
  /** Facet phân loại cho Library (đếm bỏ qua chính classes/subclasses; gồm bucket `unclassified`). */
  classFacets(q?: FaceQuery): Promise<ClassFacetDto[]>;
  /** Ngôn ngữ (Hyperglot) có ≥ 1 face trong tập lọc, kèm số face `full`/`near`; bỏ điều kiện `langs` của chính q. */
  languageList(q?: FaceQuery): Promise<LangListDto>;
  /** Ký tự thiếu của một face với một ngôn ngữ (≤ 200 + cờ cắt). */
  langCoverage(faceId: number, code: string): Promise<LangCoverageDto>;
  setClassificationOverride(over: ClassificationOverrideDto): Promise<void>;
  clearClassificationOverride(face: FaceKeyDto): Promise<void>;
  /** So sánh 2–8 face: số đo thật + percentile + Dấu chồng. Hiện ghép từ getFace; khi có command compare_faces thì đổi ở tauri.ts. */
  compareFaces(faces: FaceKeyDto[]): Promise<CompareDto>;
  /** Lưu ảnh PNG: backend mở hộp thoại Lưu (lọc .png) và ghi file; trả đường dẫn đã lưu, `null` nếu người dùng huỷ. Mock/trình duyệt: tải xuống. */
  savePng(png: Blob, suggestedName: string): Promise<string | null>;
  /** "Pair these": 2–4 font → 1–3 Combo (thử mọi cách gán vai heading/body), xếp hạng, không có điểm số. */
  pairThese(faces: FaceKeyDto[], opts?: PairTheseOpts): Promise<PairComboDto[]>;
  /** Số nhóm trùng theo loại (cùng tệp / phiên bản / định dạng); tôn trọng bộ lọc `q`. */
  duplicateSummary(q?: FaceQuery): Promise<DuplicateSummaryDto>;
  duplicateGroups(q?: FaceQuery): Promise<DuplicateGroupDto[]>;
  /** Kế hoạch giải quyết (không đổi gì): bản giữ, bản thua, lý do, đủ điều kiện Thùng rác. `q` = bộ lọc của trang. */
  resolvePreview(scope: ResolveScopeDto, q?: FaceQuery): Promise<ResolvePlanDto>;
  /** Áp dụng: từng bản thua độc lập, thành công một phần là bình thường (`done` + `failed`). */
  resolveApply(req: ResolveRequestDto): Promise<ResolveResultDto>;
  /** Hoàn tác các quyết định (mới nhất trước). `needsRescan` sau khi đưa tệp từ Thùng rác về. */
  resolveUndo(ids: number[]): Promise<ResolveResultDto>;
  resolveList(includeUndone: boolean): Promise<ResolutionDto[]>;
  duplicateIgnore(kind: DuplicateKindDto, key: string): Promise<void>;
  duplicateUnignore(kind: DuplicateKindDto, key: string): Promise<void>;
  duplicateIgnored(): Promise<IgnoredGroupDto[]>;
  /** Id nguồn, ưu tiên giữ bản nhất đứng đầu. */
  sourcePriority(): Promise<number[]>;
  /** Gửi toàn bộ danh sách sau khi kéo-thả; trả thứ tự đầy đủ mới. */
  setSourcePriority(ids: number[]): Promise<number[]>;
  pairCandidates(face: FaceKeyDto, opts: PairOptsDto): Promise<PairCandidateDto[]>;
  // Dữ liệu và hệ thống
  /** Sao lưu user.db; trả đường dẫn bản sao. */
  backupUserDb(): Promise<string>;
  previewCacheInfo(): Promise<PreviewCacheInfoDto>;
  clearPreviewCache(): Promise<void>;
  listProblems(): Promise<ProblemDto[]>;
  /** Xếp lại các font có vấn đề vào hàng đợi; trả số font. */
  retryProblems(): Promise<number>;
  // Cài đặt
  /** Chỉ khoá `ui.*` (≤ 60 ký tự sau tiền tố), giá trị ≤ 64 KB; sai → CmdError('invalid_arg'). */
  getSetting(key: string): Promise<string | null>;
  setSetting(key: string, value: string): Promise<void>;
  getSettings(): Promise<SettingsDto>;
  setSettings(patch: Partial<SettingsDto>): Promise<SettingsDto>;
  // Model và hàng đợi embed
  modelStatus(): Promise<ModelStatusDto>;
  /** Trả ngay khi bắt đầu; tiến độ, Ready, Paused, Failed qua `onEvent`. */
  modelDownload(onEvent: (e: ModelEvent) => void): Promise<void>;
  modelCancel(): Promise<void>;
  /** Đang tải thì ném CmdError('busy'). */
  modelRedownload(onEvent: (e: ModelEvent) => void): Promise<void>;
  modelRemove(): Promise<void>;
  embedStatus(): Promise<EmbedStatusDto>;
  embedPause(): Promise<void>;
  embedResume(): Promise<void>;
  embedSubscribe(onEvent: (e: EmbedEvent) => void): Promise<Unsubscribe>;
  // Kích hoạt (Plan 7)
  activationCaps(): Promise<{ method: string; supported: boolean }>;
  activationState(): Promise<ActivationStateDto[]>;
  activationPreflight(faces: FaceKeyDto[]): Promise<ActivationReportDto>;
  activationActivate(faces: FaceKeyDto[], opts?: ActivateOptions): Promise<ActivationReportDto>;
  /** "Tắt bản cũ và bật bản này": gỡ các bản xung đột (nếu ma trận cho phép) rồi bật bản của Font Atlas. Mỗi kết quả có `replaced` (id để khôi phục). */
  /** `confirmed` = xung đột người dùng đã thấy; backend từ chối (`replace_conflicts_changed`) nếu tập này đã đổi. */
  activationReplace(faces: FaceKeyDto[], confirmed: ConfirmedConflictDto[], opts?: Pick<ActivateOptions, 'scope' | 'onProgress'>): Promise<ActivationReportDto>;
  /** Các bản cũ đã gỡ và chưa khôi phục (Cài đặt → Kích hoạt). */
  activationListReplaced(): Promise<ReplacedEntryDto[]>;
  /** "Khôi phục bản cũ": đăng ký lại bản đã gỡ (cùng URL + scope) và tắt bản của Font Atlas. */
  activationRestoreReplaced(id: number): Promise<RestoreOutcomeDto>;
  activationDeactivate(faces: FaceKeyDto[], opts?: DeactivateOptions): Promise<ActivationReportDto>;
  activationActivateCollection(collectionId: number, opts?: ActivateOptions): Promise<ActivationReportDto>;
  activationDeactivateCollection(collectionId: number, opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationDeactivateAll(opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationSetKeep(on: boolean): Promise<void>;
  /** Đổi phạm vi của các font ĐANG bật (không tắt giữa chừng). */
  activationSetScope(faces: FaceKeyDto[], scope: 'session' | 'persistent'): Promise<ActivationReportDto>;
  /** Font nào đang "Có sẵn trên máy" (file giống hệt do bên khác cung cấp). Tính lại mỗi lần; `faces` kiểm thêm ngoài tập đã nhớ. */
  activationAvailability(faces?: FaceKeyDto[]): Promise<AvailabilityDto[]>;
  activationSubscribe(onEvent: (e: ActivationEvent) => void): Promise<Unsubscribe>;
}

let current: Api | null = null;
export function setApi(api: Api): void { current = api; }
export function getApi(): Api {
  if (!current) throw new Error('Api chưa được cài: gọi setApi() trước khi render');
  return current;
}
