// Mọi lệnh backend đi qua một interface; tên phương thức = tên lệnh 6a (camelCase).
// Cài đặt thật: api/tauri.ts (invoke + Channel). Cài đặt giả: api/mock.ts (dev không có Tauri, Vitest).
import type { CompareDto } from './compare';
import type { LibraryApi } from '../features/library-grid/api';
import { t } from '../i18n';
import type {
  ActivationEvent, ActivationReportDto, ActivationStateDto, AvailabilityDto, RestoreOutcomeDto, ConfirmedConflictDto, ReplacedEntryDto, BootEvent, BootStatusDto, CollectionDto, EmbedEvent, EmbedStatusDto, FaceDetailDto,
  AxisStatsDto, AxisHistogramDto, LangListDto, LangCoverageDto, BriefDto, BriefStarterDto, CaseCompareDto, PeekOpenDto, PeekOpenResultDto, CaseSettingsDto, CaseDto, CaseSummaryDto, CasePatchDto, CaseOriginDto, AddToCaseDto, ComboDto, ComboPatchDto, ComboRoleDto, NewComboDto, DuplicateGroupDto, DuplicateSummaryDto, DuplicateKindDto, IgnoredGroupDto, ResolutionDto, ResolvePlanDto, ResolveRequestDto, ResolveResultDto, ResolveScopeDto, PairComboDto, PairTheseOpts, ClassFacetDto, ClassificationOverrideDto, SimilarFamilyPageDto, FaceQuery, FaceKeyDto, FamilyStyleDto, LegacyStateDto, PreviewCacheInfoDto, ProblemDto, NameClashDto, ModelEvent, ModelStatusDto, OnboardingDto, PairCandidateDto, PairOptsDto, ProgressDto, ScanEvent, FamilyGroupDto, RemoveSourceReportDto, RebuildLibraryReportDto, CloudDownloadProgressDto, CloudDownloadReportDto, ImportProgressDto, ImportResultDto, ImportTargetDto, CaseAppDto, InstalledAppDto, AppSetEventDto, OrganizedLocationCheckDto, OrganizedStatusDto, OrganizePlanDto, OrganizeResultDto, OrganizeProgressDto, OrganizeScopeDto, ResortPlanDto, ResortResultDto, PackInfoDto, PathRemapDto, CaseActivationDto,
  ScanStatusDto, ScopeDto, SettingsDto, SimilarHitDto, SimilarQuery, SourceDto, SystemTagGroupDto, TagDto, UiStateDto, Unsubscribe, AutoLogDto, AutoStatusDto, AutoTargetDto, VfPolicyDto,
} from './types';

import type { DeskSuggestReqDto, DeskPageDto, SuggestPartnersReqDto, PartnerPageDto, SuggestCombosReqDto, ComboSuggestionDto, RoleRefDto, RoleFitDto, RecentViewDto, FigmaImportDto, FontNameDto, FontMatchDto, PendingImportDto } from '../bindings';

export * from './types';

export type ActivateOptions = { scope?: ScopeDto | null; proceed?: boolean; onProgress?(p: ProgressDto): void };
export type DeactivateOptions = { force?: boolean; onProgress?(p: ProgressDto): void };

export interface Api extends LibraryApi {
  // Khởi động và onboarding
  startupStatus(): Promise<BootStatusDto>;
  bootSubscribe(onEvent: (e: BootEvent) => void): Promise<Unsubscribe>;
  onboardingState(): Promise<OnboardingDto>;
  setOnboardingStep(step: string | null): Promise<void>;
  /** Bật/tắt kính thật (Liquid Glass) sau webview cho màn hướng dẫn. Trả true khi kính đang hiện; false = không có (giữ vibrancy). */
  setWindowGlass(on: boolean): Promise<boolean>;
  legacyImport(): Promise<LegacyStateDto>;
  legacyRetry(): Promise<LegacyStateDto>;
  // Nguồn và quét
  listSources(): Promise<SourceDto[]>;
  /** Ném CmdError('nested_source', …, { existingId, existingPath }) khi nằm trong/chứa nguồn có sẵn. */
  addSource(path: string): Promise<SourceDto>;
  /**
   * Gỡ nguồn. Trả số liệu đã làm. KHÔNG xoá dữ liệu người dùng: Dự án/Collection/Tag/Yêu thích/ghi đè phân loại giữ nguyên
   * (font chỉ có ở nguồn này hiện `missing` trong Dự án, ẩn khỏi lưới; thêm lại nguồn thì khớp lại theo hash).
   * Font đang bật không tự tắt (`activeFonts` để cảnh báo).
   */
  removeSource(sourceId: number): Promise<RemoveSourceReportDto>;
  /** Số liệu thật cho câu xác nhận gỡ nguồn — chỉ đọc. */
  removeSourcePreview(sourceId: number): Promise<RemoveSourceReportDto>;
  rescanSources(sourceIds?: number[]): Promise<void>;
  /**
   * "Dựng lại thư viện" (Cài đặt ▸ Dữ liệu, CẦN hộp xác nhận): dừng mọi lần quét, xoá toàn bộ dữ liệu thư viện (font, vị trí,
   * phân tích, embedding, ngôn ngữ) rồi quét lại mọi nguồn. KHÔNG đụng dữ liệu người dùng (Dự án, Collection, Tag, Yêu thích,
   * nguồn, ghi đè phân loại; gán theo hash nên khớp lại khi font quay về). Font hiện dần khi quét; phân tích và embedding
   * tính lại ở nền nên có thể mất lâu với thư viện lớn. Ném CmdError('busy') nếu có nguồn chưa dừng quét kịp.
   * BẮT BUỘC cho hộp xác nhận (05 viết chữ, vi + en): nói rõ (1) font hiện dần khi quét lại, (2) phân tích và embedding sẽ được TÍNH LẠI
   * ở nền và có thể mất RẤT LÂU với thư viện lớn, (3) Dự án/Collection/Tag/Yêu thích và các nguồn KHÔNG bị đụng. Nút xác nhận không
   * là nút mặc định; nêu số `filesRemoved` nếu có.
   */
  rebuildLibrary(): Promise<RebuildLibraryReportDto>;
  /** Mở Finder tại thư mục của nguồn (đường dẫn lấy từ Store). Ném CmdError('source_unavailable') khi thư mục không còn (ổ chưa mount). */
  revealSource(sourceId: number): Promise<void>;
  /**
   * "Tải về máy": yêu cầu hệ điều hành tải các font còn là placeholder trên ổ cloud của nguồn (tối đa 4 cùng lúc), báo tiến độ,
   * xong thì quét lại nguồn. Lỗi từng font không làm hỏng phần còn lại (`firstFailureCode`). Ném CmdError('busy') nếu nguồn đang tải,
   * 'source_unavailable' nếu thư mục không còn.
   */
  /** G8: thả file/thư mục từ Finder vào Dự án/Case. Một lượt tại một thời điểm ('busy'); `added` = đúng face mới vào đích (Hoàn tác gỡ đúng danh sách này). */
  importPathsInto(paths: string[], target: ImportTargetDto, onProgress?: (p: ImportProgressDto) => void): Promise<ImportResultDto>;
  importPathsCancel(): Promise<boolean>;
  /** B5: app gắn với Case (bundleId là khoá). `caseSetApps` THAY TOÀN BỘ danh sách; lỗi invalid_arg/not_found thì danh sách cũ giữ nguyên. */
  /** B7 Organized Library: chép (không di chuyển) font vào thư viện riêng `Class/Họ/`; mọi lệnh theo hợp đồng 02. */
  organizedCheckLocation(path: string): Promise<OrganizedLocationCheckDto>;
  organizedCreate(path: string): Promise<OrganizedStatusDto>;
  organizedStatus(): Promise<OrganizedStatusDto>;
  organizedPlan(scope: OrganizeScopeDto): Promise<OrganizePlanDto>;
  organizedImport(scope: OrganizeScopeDto, onProgress?: (p: OrganizeProgressDto) => void): Promise<OrganizeResultDto>;
  organizedCancel(): Promise<boolean>;
  organizedResortPreview(): Promise<ResortPlanDto>;
  organizedResortApply(): Promise<ResortResultDto>;
  organizedResortUndo(batchId: number): Promise<ResortResultDto>;
  organizedSnapshotNow(): Promise<string>;
  packInspect(path: string): Promise<PackInfoDto>;
  packRestoreStage(path: string, remap: PathRemapDto[]): Promise<PackInfoDto>;
  packRestorePending(): Promise<PackInfoDto | null>;
  packRestoreCancel(): Promise<boolean>;
  caseApps(collectionId: number): Promise<CaseAppDto[]>;
  caseSetApps(collectionId: number, apps: CaseAppDto[]): Promise<void>;
  /** B5: app đã cài (quét /Applications…), sắp theo tên; `running` = đang chạy lúc quét. */
  installedApps(): Promise<InstalledAppDto[]>;
  sourceDownloadCloud(sourceId: number, onProgress?: (p: CloudDownloadProgressDto) => void): Promise<CloudDownloadReportDto>;
  /** Huỷ lần tải đang chạy của nguồn (font đang tải dở chạy nốt); false nếu không có lần nào. */
  sourceDownloadCancel(sourceId: number): Promise<boolean>;
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
  /** R6: đổi tên Tag, giữ gán thẻ. Lỗi: `invalid_arg` (tên trống / tiền tố style:|mood:|use: ngoài danh mục / Tag hệ thống → tên thường), `conflict` (trùng Tag khác, không gộp ngầm), `not_found`. */
  renameTag(id: number, name: string): Promise<TagDto>;
  /** R6: xoá Tag + mọi gán thẻ; trả số face đã bị bỏ thẻ (để nêu trong xác nhận). Không đụng font/Collection/Dự án. */
  deleteTag(id: number): Promise<number>;
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
  /** R3: các nhóm family của truy vấn, đúng thứ tự `queryFaces({ ...q, groupByFamily: true })`: `[start, start+count)` là offset của nhóm. */
  familyGroups(q?: FaceQuery): Promise<FamilyGroupDto[]>;
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
  /** Đặt font vào vai; vai chưa có thì được tạo. `face` null = chỉ gỡ font, VAI Ở LẠI (rỗng giữ chỗ): muốn bỏ vai thì `removeComboRole`.
   *  Vai là chuỗi tuỳ ý (1–40 ký tự, không phân biệt hoa thường trong Combo); 7 khoá preset vẫn dùng như cũ.
   *  Lỗi: `role_name_invalid`, `role_limit` (tối đa 30 vai). */
  setComboSlot(comboId: number, role: ComboRoleDto, face: FaceKeyDto | null, note?: string, style?: unknown): Promise<ComboDto>;
  /** Thêm vai rỗng giữ chỗ ở cuối `roles`. Lỗi: `role_name_invalid`, `role_duplicate`, `role_limit`. */
  addComboRole(comboId: number, name: string): Promise<ComboDto>;
  /** Bỏ hẳn vai (kèm font đang đặt ở đó; font vẫn ở trong Case). Lỗi: `role_not_found`. */
  removeComboRole(comboId: number, role: ComboRoleDto): Promise<ComboDto>;
  /** Đổi tên vai, giữ font/ghi chú/kiểu/vị trí. Lỗi: `role_not_found`, `role_name_invalid`, `role_duplicate`. */
  renameComboRole(comboId: number, role: ComboRoleDto, newName: string): Promise<ComboDto>;
  /** Sắp lại thứ tự: `roles` phải đúng bằng tập `ComboDto.roles` hiện có. Lỗi: `role_set_mismatch`. */
  reorderComboRoles(comboId: number, roles: ComboRoleDto[]): Promise<ComboDto>;
  createCombos(caseId: number, combos: NewComboDto[]): Promise<ComboDto[]>;
  /** G5: nguồn của Picker. `source` ∈ fits_role | similar | pinned (Pairs dùng `suggestPartners`). Lỗi: `source_unknown`, `anchor_required`, `case_required`, `role_unknown`. */
  deskSuggest(req: DeskSuggestReqDto): Promise<DeskPageDto>;
  /** G5: gợi ý đối tác theo vai + phong cách pair (G4). `styles` rỗng = mọi phong cách. Lỗi: `style_unknown`. */
  suggestPartners(anchor: FaceKeyDto, req: SuggestPartnersReqDto): Promise<PartnerPageDto>;
  /** G6: gợi ý combo từ thư viện. Cùng `seed` cùng kết quả. Lỗi: `style_unknown`, `roles_invalid`, `locked_role_unknown`, `locked_duplicate`, `scope_unknown`, `case_required`. */
  suggestCombos(req: SuggestCombosReqDto): Promise<ComboSuggestionDto[]>;
  /** Mức hợp vai của một font cho một vai (G4). */
  roleFit(face: FaceKeyDto, role: RoleRefDto): Promise<RoleFitDto>;
  /** Font vừa xem (lưu ở backend, sống qua các lần mở app). */
  recentViews(limit: number): Promise<RecentViewDto[]>;
  /** Ghi một lần xem (mở Detail/Desk, không ghi khi lướt lưới). */
  recordView(face: FaceKeyDto): Promise<void>;
  /** Xoá danh sách font vừa xem; trả số dòng đã xoá. */
  clearRecentViews(): Promise<number>;
  /** Nhập từ Figma (B6). Phân tích văn bản dán (JSON hoặc link fontatlas://); chỉ đọc. Lỗi: `import_empty|format|version|too_large|too_many_fonts`. */
  parseFigmaImport(text: string): Promise<FigmaImportDto>;
  /** Khớp tên font Figma với thư viện (≤200). Kết quả cùng thứ tự đầu vào; không điểm số. */
  resolveFonts(items: FontNameDto[]): Promise<FontMatchDto[]>;
  /** Link fontatlas:// đã nhận (kể cả mở lạnh) chờ giao diện lấy; lấy xong là rỗng. */
  takePendingImports(): Promise<PendingImportDto[]>;
  /** Báo khi có link mới tới lúc app đang chạy. */
  figmaImportSubscribe(onPending: () => void): Promise<Unsubscribe>;
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
  /** G7: các cặp font Variable ↔ static trùng tên PostScript (chỉ đọc; không đổi file). */
  nameClashes(): Promise<NameClashDto[]>;
  /** Xếp lại các font có vấn đề vào hàng đợi; trả số font. */
  retryProblems(): Promise<number>;
  // Cài đặt
  /** Chỉ khoá `ui.*` (≤ 60 ký tự sau tiền tố), giá trị ≤ 64 KB; sai → CmdError('invalid_arg'). */
  getSetting(key: string): Promise<string | null>;
  setSetting(key: string, value: string): Promise<void>;
  getSettings(): Promise<SettingsDto>;
  setSettings(patch: Partial<SettingsDto>): Promise<SettingsDto>;
  // Model và hàng đợi embed
  // Bật font tự động (0.3.0): trạng thái từng app, công tắc chung, cài/gỡ panel/script Adobe, nhật ký.
  autoStatus(): Promise<AutoStatusDto>;
  autoSetEnabled(on: boolean): Promise<AutoStatusDto>;
  autoInstall(target: AutoTargetDto): Promise<AutoStatusDto>;
  autoRemove(target: AutoTargetDto): Promise<AutoStatusDto>;
  /** Xoá khoá ký riêng của máy (chỉ khi panel Illustrator đã gỡ); bản cô lập từ chối. */
  autoForgetKey(): Promise<AutoStatusDto>;
  autoProbe(): Promise<AutoStatusDto>;
  autoLog(limit?: number): Promise<AutoLogDto[]>;
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
  /**
   * "Bật tất cả" của một Dự án: font không `missing` đi qua đúng luồng `activationActivate` (xung đột → `needsConfirmation`,
   * gọi lại với `proceed: true`); font `missing` liệt kê ở `skippedMissing` (không tính là lỗi). Kích hoạt thủ công, không gắn lý do "dự án".
   */
  activationActivateCase(caseId: number, opts?: ActivateOptions): Promise<CaseActivationDto>;
  activationDeactivateCollection(collectionId: number, opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationDeactivateAll(opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationSetKeep(on: boolean): Promise<void>;
  /** G7: lựa chọn đã nhớ cho cặp font variable (VF) ↔ static trùng tên PostScript; `ask` = hỏi mỗi lần (mặc định). */
  activationVfPolicy(): Promise<VfPolicyDto>;
  activationSetVfPolicy(policy: VfPolicyDto): Promise<void>;
  /** Đổi phạm vi của các font ĐANG bật (không tắt giữa chừng). */
  activationSetScope(faces: FaceKeyDto[], scope: 'session' | 'persistent'): Promise<ActivationReportDto>;
  /** Font nào đang "Có sẵn trên máy" (file giống hệt do bên khác cung cấp). Tính lại mỗi lần; `faces` kiểm thêm ngoài tập đã nhớ. */
  activationAvailability(faces?: FaceKeyDto[]): Promise<AvailabilityDto[]>;
  activationSubscribe(onEvent: (e: ActivationEvent) => void): Promise<Unsubscribe>;
}

let current: Api | null = null;
export function setApi(api: Api): void { current = api; }
export function getApi(): Api {
  if (!current) throw new Error(t('api.notInstalled'));
  return current;
}
