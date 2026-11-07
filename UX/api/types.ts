export type { ClassFacetDto, ClassificationDto, ClassificationOverrideDto, AxisStatsDto, SimilarFamilyDto, SimilarFamilyPageDto, RawAxisRangeDto } from '../bindings';
// Kiểu IPC: lấy từ UX/bindings (ts-rs, sinh từ Rust). File này chỉ đặt lại tên cho gọn và thêm kiểu phía giao diện.
import type {
  PreviewCacheInfoDto, ProblemDto,
  ActivationEvent, ActivationReportDto, ActivationStateDto, AutoDeactivatedDto, AxisReadingDto, BootEvent, BootStatusDto, CollectionDto,
  ConflictDto, DesiredStateDto, EmbedEvent, EmbedStatusDto, ErrorInfoDto, FaceDetailDto, FaceKeyDto, FacePageDto, FamilyStyleDto, FileOutcomeDto,
  FileResultDto, FontManagerDto, GroupWeightsDto, LegacyStateDto, ModelEvent, ModelStatusDto, OnboardingDto, PairCandidateDto, PairLabelDto,
  PairModeDto, PairOptsDto, PairRoleDto, ProgressDto, ReasonDto, ReconcileReportDto, ScanEvent, ScanStatusDto, ScopeDto, SettingsDto,
  SimilarHitDto, SimilarSourceDto, SourceDto, TagDto, UiStateDto,
} from '../bindings';
import type { FacePage, FaceQuery, FaceRow } from '../features/library-grid/types';

export type {
  ActivationEvent, ActivationReportDto, ActivationStateDto, AutoDeactivatedDto, AxisReadingDto, BootEvent, BootStatusDto, CollectionDto,
  ConflictDto, DesiredStateDto, EmbedEvent, EmbedStatusDto, ErrorInfoDto, FaceDetailDto, FaceKeyDto, FacePageDto, FamilyStyleDto, FileOutcomeDto,
  FileResultDto, FontManagerDto, GroupWeightsDto, LegacyStateDto, ModelEvent, ModelStatusDto, OnboardingDto, PairCandidateDto, PairLabelDto,
  PairModeDto, PairOptsDto, PairRoleDto, ProgressDto, ReasonDto, ReconcileReportDto, ScanEvent, ScanStatusDto, ScopeDto, SettingsDto,
  SimilarHitDto, SimilarSourceDto, SourceDto, TagDto, UiStateDto, FacePage, FaceQuery, FaceRow,
};
export type { PreviewCacheInfoDto, ProblemDto };
export type ModelState = ModelStatusDto['state'];
export type PairRole = PairRoleDto;
export type PairMode = PairModeDto;
export type SimilarSource = SimilarSourceDto;

/** Lỗi lệnh (6a: `CmdError {code, message, detail?}`); lệnh thật reject bằng object thường, lớp này gói lại để `instanceof` được. */
export type CmdErrorCode =
  | 'store_unavailable' | 'not_found' | 'not_comparable' | 'invalid_arg' | 'nested_source' | 'busy'
  | 'model_missing' | 'cancelled' | 'io' | 'insufficient_disk' | 'activation_unsupported' | 'internal';

export class CmdError extends Error {
  constructor(public code: CmdErrorCode | (string & {}), message: string, public detail?: unknown) {
    super(message);
  }
}

/** Tuỳ chọn Similar phía giao diện: phần thiếu được điền mặc định của 6a (trọng số 1, useImage, k 12). */
export type SimilarQuery = { k?: number; useImage?: boolean; viOnly?: boolean; weights?: GroupWeightsDto; sourceIds?: number[]; weightRange?: [number, number] | null };

export const DEFAULT_UI_STATE: UiStateDto = {
  sidebarCollapsed: false, openSections: ['col', 'src', 'tag'], layout: 'grid', sampleSize: 34, infoExpanded: false,
};

/** Lệnh model/embed có Channel dùng chung kiểu callback phía giao diện. */
export type Unsubscribe = () => void;
export type { CompareDto, CompareFace, CompareMetric, CompareMetricKey } from './compare';
export type { PairComboDto } from '../bindings/PairComboDto';
export type PairTheseOpts = { projectVi?: boolean; maxCombos?: number };
