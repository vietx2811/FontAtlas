import type { RawAxisRangeDto } from '../../bindings';
// Kiểu dữ liệu lưới thư viện — gương fontatlas-store (serde camelCase) trên dev/mvp.

export type FaceQuery = {
  rawAxisRanges?: RawAxisRangeDto[];
  text?: string; vi?: 'full' | 'supported'; license?: string[];
  variable?: boolean; italic?: boolean; sourceIds?: number[];
  favoritesOnly?: boolean; collectionId?: number; tagIds?: number[];
  includeMissing?: boolean; sort?: 'family' | 'weight' | 'added' | 'match';
  descending?: boolean; offset?: number; limit?: number;
  /** 6a (b2cc0fd): chỉ các face này, ≤ 2000; sort='match' giữ thứ tự đưa vào. Dùng cho "Gần đây"/"Đang bật". */
  faceKeys?: Array<{ hashHex: string; faceIndex: number }>;
  /** D1 (feat/classify): lọc theo phân loại hiệu lực; OR trong nhóm, AND giữa hai nhóm. */
  classes?: string[]; subclasses?: string[]; traits?: string[];
};

export type FaceRow = {
  faceId: number; hashHex: string; faceIndex: number; family: string;
  subfamily: string | null; fullName: string | null;
  weightClass: number | null; widthClass: number | null;
  isItalic: boolean; isVariable: boolean; viStatus: string;
  supportsVi: boolean; viMissing: number; licenseClass: string;
  path: string | null; fileSize: number; missing: boolean;
  isFavorite: boolean; addedAt: number;
};

export type FacePage = { total: number; offset: number; items: FaceRow[] };
export type GridQuery = Omit<FaceQuery, 'offset' | 'limit'>;

export class ContractError extends Error {}

type Kind = 'number' | 'string' | 'boolean' | 'string|null' | 'number|null';
// Record<keyof FaceRow, …> buộc bảng đủ khoá: thêm trường vào FaceRow mà quên ở đây là lỗi tsc.
const ROW_SCHEMA: Record<keyof FaceRow, Kind> = {
  faceId: 'number', hashHex: 'string', faceIndex: 'number', family: 'string',
  subfamily: 'string|null', fullName: 'string|null',
  weightClass: 'number|null', widthClass: 'number|null',
  isItalic: 'boolean', isVariable: 'boolean', viStatus: 'string',
  supportsVi: 'boolean', viMissing: 'number', licenseClass: 'string',
  path: 'string|null', fileSize: 'number', missing: 'boolean',
  isFavorite: 'boolean', addedAt: 'number',
};

function matches(v: unknown, kind: Kind): boolean {
  const [base, nullable] = kind.split('|');
  return (nullable === 'null' && v === null) || typeof v === base;
}

/** Đọc một FaceRow từ JSON của backend; sai hợp đồng → ContractError kèm đường dẫn trường. */
export function parseFaceRow(x: unknown, path = ''): FaceRow {
  if (typeof x !== 'object' || x === null) throw new ContractError(`${path || 'row'}: expected object`);
  const o = x as Record<string, unknown>;
  for (const [k, kind] of Object.entries(ROW_SCHEMA) as Array<[keyof FaceRow, Kind]>) {
    if (!(k in o)) throw new ContractError(`${path}${k}: missing`);
    if (!matches(o[k], kind)) throw new ContractError(`${path}${k}: expected ${kind}`);
  }
  for (const k of Object.keys(o)) {
    if (!(k in ROW_SCHEMA)) throw new ContractError(`${path}unexpected key ${k}`);
  }
  return o as FaceRow;
}

export function parseFacePage(x: unknown): FacePage {
  if (typeof x !== 'object' || x === null) throw new ContractError('page: expected object');
  const o = x as Record<string, unknown>;
  if (typeof o.total !== 'number') throw new ContractError('total: expected number');
  if (typeof o.offset !== 'number') throw new ContractError('offset: expected number');
  if (!Array.isArray(o.items)) throw new ContractError('items: expected array');
  return { total: o.total, offset: o.offset, items: o.items.map((it, i) => parseFaceRow(it, `items[${i}].`)) };
}

export const PX_BUCKETS = [32, 48, 64, 96] as const;
export type PxBucket = typeof PX_BUCKETS[number];
export const DEFAULT_SAMPLE = 'Hamburgefonts Ăn phở';

/** Một nửa canvas Rust (ảnh 2×): W = 13·px, H = ⌊7·px/3⌋. */
export function cssSize(px: PxBucket): { width: number; height: number } {
  return { width: (13 * px) / 2, height: Math.floor((7 * px) / 3) / 2 };
}

/** Bề rộng tối thiểu của ô theo cỡ chữ mẫu (Plan 6b T6): 140 + size × 2.6 px. */
export const cellMinWidth = (fontSize: number): number => 140 + fontSize * 2.6;

/** Bucket ảnh gần nhất với cỡ hiển thị: ảnh vẽ ở px trên canvas 2× nên cỡ css = px/2. */
export function bucketFor(fontSize: number): PxBucket {
  return PX_BUCKETS.reduce((best, b) => (Math.abs(b - 2 * fontSize) < Math.abs(best - 2 * fontSize) ? b : best));
}

/**
 * Hộp ảnh hiển thị: co ảnh bucket về cỡ chữ người dùng chọn, GIỮ NGUYÊN tỉ lệ của PNG (rộng 13·px, cao ⌊7·px/3⌋).
 * Không làm tròn từng chiều để tỉ lệ không lệch; khung chứa sẽ cắt phần tràn chứ không bóp ảnh.
 */
export function previewBox(px: PxBucket, fontSize: number): { width: number; height: number } {
  const width = 13 * fontSize;
  return { width, height: (width * Math.floor((7 * px) / 3)) / (13 * px) };
}
