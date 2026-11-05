// DTO của Plan 6a (bảng lệnh cuối, commit 38d3d62). Khi UX/bindings (ts-rs) có thật,
// file này chỉ còn re-export; tên và hình dạng giữ nguyên.
import type { FacePage, FaceQuery, FaceRow } from '../features/library-grid/types';

export type { FacePage, FaceQuery, FaceRow };

export type FaceKeyDto = { hashHex: string; faceIndex: number };

export type CmdErrorCode =
  | 'store_unavailable' | 'not_found' | 'not_comparable' | 'invalid_arg' | 'nested_source' | 'busy'
  | 'model_missing' | 'cancelled' | 'io' | 'insufficient_disk' | 'activation_unsupported' | 'internal';

export class CmdError extends Error {
  constructor(public code: CmdErrorCode, message: string, public detail?: unknown) {
    super(message);
  }
}

export type SourceDto = {
  id: number; kind: string; name: string; path: string | null; enabled: boolean;
  available: boolean; faceCount: number; lastScan: unknown | null; scanning: boolean;
};

export type CollectionDto = { id: number; name: string; color: string | null; itemCount: number };
export type TagDto = { id: number; name: string; faceCount: number };

export type FamilyStyleDto = {
  face: FaceKeyDto; subfamily: string | null; weight: number | null; italic: boolean;
  instance?: { name: string; coords: Array<{ tag: string; value: number }> } | null;
};

export type UiStateDto = {
  sidebarCollapsed: boolean; openSections: string[]; layout: string; sampleSize: number; infoExpanded: boolean;
};
export const DEFAULT_UI_STATE: UiStateDto = {
  sidebarCollapsed: false, openSections: ['col', 'src', 'tag'], layout: 'grid', sampleSize: 34, infoExpanded: false,
};

export type AxisQuality = 'measured' | 'estimated' | 'failed';
export type AxisReadingDto = {
  key: string; value: number | string | null; quality: AxisQuality; reason: string | null; method: string | null; percentile: number | null;
};

export type SimilarSource = 'axes' | 'axes_and_image' | 'image';
/** `distance` có trong DTO nhưng UI không hiển thị điểm hay phần trăm (spec). */
export type SimilarHitDto = { face: FaceKeyDto; row: FaceRow; distance: number; source: SimilarSource; same: string[]; differ: string[] };
export type SimilarOptsDto = { k: number; useImage?: boolean; viOnly?: boolean };

export type PairRole = 'heading' | 'body' | 'code';
export type PairMode = 'safe' | 'controlled' | 'experimental';
export type PairOptsDto = { role: PairRole; mode: PairMode | null; projectVi: boolean; k: number; collectionId: number | null; lang: 'vi' | 'en' };
export type PairSymbol = '●' | '◐' | '◆' | '○' | '◑';
export type PairLabelDto = { axis: string; relation: string; symbol: PairSymbol; delta: number | null; a: number | string | null; b: number | string | null; roleReversed: boolean };
/** Không có trường điểm số nào (6a). */
export type PairCandidateDto = {
  face: FaceKeyDto; row: FaceRow; gate: string; labels: PairLabelDto[]; clashes: string[]; bodyWarnings: string[];
  headingOverflow: boolean; insufficientData: boolean; weightHint: string | null; block: string | null; sentences: string[]; summary: string;
};

export type FaceDetailDto = {
  row: FaceRow; summary: string;
  /** [6a: thêm] Phân loại hiển thị ở dòng dưới tên (Sans, Serif, Mono…). */
  classification: string | null;
  source: 'axes' | 'axes_and_image'; hasImage: boolean;
  file: { format: string; container: string | null; outline: 'glyf' | 'cff' | 'cff2' | 'none'; numGlyphs: number; sizeBytes: number; path: string | null };
  metadata: {
    license: string | null; licenseUrl: string | null; designer: string | null; manufacturer: string | null;
    version: string | null; copyright: string | null; vendorUrl: string | null;
    variableAxes: Array<{ tag: string; min: number; default: number; max: number }>;
    namedInstances: Array<{ name: string; coords: Array<{ tag: string; value: number }> }>;
    scripts: string[]; opszRange: [number, number] | null; features: { gsub: string[]; gpos: string[] };
  };
  vietnamese: {
    status: string; supports: boolean; missing: string[];
    /** [6a: thêm] Tổng số ký tự tiếng Việt cần có, để vẽ thanh X/N. */
    total: number;
    precomposedComposite: boolean; gposMark: boolean; gposMkmk: boolean; gsubCcmp: boolean; dongSign: boolean;
    stackedMarksTopEm: number | null; stackedMarksOverflow: boolean; stackStyle: string | null; markWeightRatio: AxisReadingDto;
  };
  axes: AxisReadingDto[];
  similarTop: SimilarHitDto[];
};

/** Gương ActivationStateDto của Plan 7 (camelCase). */
export type ActivationStateDto = {
  hashHex: string; state: 'active' | 'inactive'; scope: string; status: string;
  reasons: Array<string | { collection: number }>; activePath: string | null; conflictPs: string | null; lastError: string | null;
};

export type SettingsDto = {
  embedEnabled: boolean; licenseNoticeAck: boolean; uiLang: 'vi' | 'en'; sampleText: string;
  projectViDefault: boolean; keepActiveOnExit: boolean; rescanOnLaunch: boolean;
};
