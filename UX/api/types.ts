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

export type FaceDetailDto = {
  row: FaceRow; summary: string;
  /** [6a: thêm] Phân loại hiển thị ở dòng dưới tên (Sans, Serif, Mono…). */
  classification: string | null; source: 'axes' | 'axes_and_image'; hasImage: boolean;
  file: { format: string; container: string | null; outline: 'glyf' | 'cff' | 'cff2' | 'none'; numGlyphs: number; sizeBytes: number; path: string | null };
  metadata: {
    license: string | null; licenseUrl: string | null; designer: string | null; manufacturer: string | null;
    version: string | null; copyright: string | null; vendorUrl: string | null;
    variableAxes: Array<{ tag: string; min: number; default: number; max: number }>;
    namedInstances: Array<{ name: string; coords: Array<{ tag: string; value: number }> }>;
    scripts: string[]; opszRange: [number, number] | null; features: { gsub: string[]; gpos: string[] };
  };
  vietnamese: { status: string; supports: boolean; missing: string[]; stackedMarksOverflow: boolean; stackStyle: string | null };
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
