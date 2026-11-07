// Mọi lệnh backend đi qua một interface; tên phương thức = tên lệnh 6a (camelCase).
// Cài đặt thật: api/tauri.ts (invoke + Channel). Cài đặt giả: api/mock.ts (dev không có Tauri, Vitest).
import type { CompareDto } from './compare';
import type { LibraryApi } from '../features/library-grid/api';
import type {
  ActivationEvent, ActivationReportDto, ActivationStateDto, BootEvent, BootStatusDto, CollectionDto, EmbedEvent, EmbedStatusDto, FaceDetailDto,
  AxisStatsDto, DuplicateGroupDto, DuplicateSummaryDto, PairComboDto, PairTheseOpts, ClassFacetDto, ClassificationOverrideDto, SimilarFamilyPageDto, FaceQuery, FaceKeyDto, FamilyStyleDto, LegacyStateDto, PreviewCacheInfoDto, ProblemDto, ModelEvent, ModelStatusDto, OnboardingDto, PairCandidateDto, PairOptsDto, ProgressDto, ScanEvent,
  ScanStatusDto, ScopeDto, SettingsDto, SimilarHitDto, SimilarQuery, SourceDto, TagDto, UiStateDto, Unsubscribe,
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
  tagsOf(face: FaceKeyDto): Promise<TagDto[]>;
  createTag(name: string): Promise<TagDto>;
  tagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  untagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  revealInFinder(face: FaceKeyDto): Promise<void>;
  /** Mở Finder đúng một đường dẫn đã lưu trong thư viện (nhóm cùng tệp có nhiều đường dẫn cho một face). */
  revealLocation(path: string): Promise<void>;
  getUiState(): Promise<UiStateDto>;
  setUiState(state: UiStateDto): Promise<void>;
  // Phân tích
  getFace(face: FaceKeyDto, lang: 'vi' | 'en'): Promise<FaceDetailDto>;
  similar(face: FaceKeyDto, opts?: SimilarQuery): Promise<SimilarHitDto[]>;
  similarFamilies(face: FaceKeyDto, opts?: SimilarQuery, q?: FaceQuery): Promise<SimilarFamilyPageDto>;
  axisStats(q?: FaceQuery): Promise<AxisStatsDto[]>;
  /** Facet phân loại cho Library (đếm bỏ qua chính classes/subclasses; gồm bucket `unclassified`). */
  classFacets(q?: FaceQuery): Promise<ClassFacetDto[]>;
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
  activationDeactivate(faces: FaceKeyDto[], opts?: DeactivateOptions): Promise<ActivationReportDto>;
  activationActivateCollection(collectionId: number, opts?: ActivateOptions): Promise<ActivationReportDto>;
  activationDeactivateCollection(collectionId: number, opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationDeactivateAll(opts?: Pick<DeactivateOptions, 'onProgress'>): Promise<ActivationReportDto>;
  activationSetKeep(on: boolean): Promise<void>;
  activationSubscribe(onEvent: (e: ActivationEvent) => void): Promise<Unsubscribe>;
}

let current: Api | null = null;
export function setApi(api: Api): void { current = api; }
export function getApi(): Api {
  if (!current) throw new Error('Api chưa được cài: gọi setApi() trước khi render');
  return current;
}
