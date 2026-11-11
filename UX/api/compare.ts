import type { FaceExtrasDto } from '../bindings/FaceExtrasDto';
import type { FamilyStructureDto } from '../bindings/FamilyStructureDto';
import type { LicenseNoteDto } from '../bindings/LicenseNoteDto';
import type { AxisReadingDto, ClassificationDto, FaceDetailDto, FaceKeyDto, FaceRowDto } from '../bindings';
import { t, getLocale } from '../i18n';

/** Các số đo hiện trong Compare, theo thứ tự hàng. Khoá = `AxisReadingDto.key` canonical. */
export const COMPARE_METRICS = ['x_height_em', 'cap_height_em', 'contrast', 'contrast_h', 'slant_deg', 'stress_deg', 'aperture', 'serif_foot'] as const;
export type CompareMetricKey = typeof COMPARE_METRICS[number];

export type CompareMetric = {
  key: string; value: number | null; unit: string | null;
  quality: 'measured' | 'estimated' | 'failed'; reason: string | null; percentile: number | null;
  /** Chênh lệch so với mốc do backend tính (tuyệt đối + %); `null` ở mốc hoặc khi thiếu số; `undefined` = chưa có (UI tự tính). */
  delta?: { abs: number | null; pct: number | null } | null;
};
export type CompareFace = {
  face: FaceKeyDto; row: FaceRowDto; classification: ClassificationDto;
  metrics: Record<string, CompareMetric>;
  vi: {
    supports: boolean; missing: number; total: number; stackedMarksOverflow: boolean; stackedMarksTopEm: number | null;
    /** % em dấu chồng vượt hhea.ascender (0 khi không vượt) / cao hơn cap height; `null` khi backend chưa tính. */
    overAscenderPctEm?: number | null; overCapPctEm?: number | null;
  };
  /** Tên giấy phép hiển thị (kind từ backend hoặc chuỗi license của font). */
  license: string | null; designer: string | null;
  /** D2: vendor, năm, nhóm OpenType, kerning, cấu trúc style, fsType. `null` khi nguồn dữ liệu không có (phân tích cũ). */
  extras?: FaceExtrasDto | null; family?: FamilyStructureDto | null; licenseNote?: LicenseNoteDto | null;
};
export type CompareDto = { faces: CompareFace[]; /** Face không đọc được (kèm lý do); các cột còn lại vẫn so sánh. */ failed: Array<{ face: FaceKeyDto; reason: string }> };
export const COMPARE_MIN = 2;
export const COMPARE_MAX = 8;

const asQuality = (q: string): CompareMetric['quality'] => (q === 'measured' || q === 'estimated' ? q : 'failed');
function metricOf(key: string, a: AxisReadingDto | undefined): CompareMetric {
  if (!a || a.rawValue === null || !Number.isFinite(a.rawValue)) {
    return { key, value: null, unit: a?.unit ?? null, quality: 'failed', reason: a?.reason ?? null, percentile: null };
  }
  return { key, value: a.rawValue, unit: a.unit, quality: asQuality(a.quality), reason: a.reason, percentile: Number.isFinite(a.percentile) ? a.percentile : null };
}

/** Ghép chi tiết từng face thành mô hình cho màn Compare. Chỉ dùng số đo thật; thiếu → `failed`, không điền số. Chịu được trường thiếu của dữ liệu thật. */
export function buildCompare(details: FaceDetailDto[], failed: CompareDto['failed'] = []): CompareDto {
  return {
    failed,
    faces: details.map(d => {
      const axes = d.axes ?? [];
      const vi = d.vietnamese;
      const c = d.classificationInfo;
      return {
        face: { hashHex: d.row.hashHex, faceIndex: d.row.faceIndex }, row: d.row,
        classification: { ...c, usage: c?.usage ?? [], signals: c?.signals ?? [] },
        metrics: Object.fromEntries(COMPARE_METRICS.map(k => [k, metricOf(k, axes.find(a => a.key === k))])),
        vi: { supports: vi?.supports ?? false, missing: vi?.missing?.length ?? 0, total: vi?.total ?? 0, stackedMarksOverflow: vi?.stackedMarksOverflow ?? false, stackedMarksTopEm: vi?.stackedMarksTopEm ?? null },
        license: d.metadata?.license ?? null, designer: d.metadata?.designer ?? null,
      };
    }),
  };
}

/** Triển khai chung (tauri + mock): đọc từng face bằng getFace rồi ghép. Face lỗi được ghi vào `failed`; còn < 2 face đọc được thì lỗi. */
export async function composeCompare(api: { getFace(f: FaceKeyDto, lang: 'vi' | 'en'): Promise<FaceDetailDto> }, faces: FaceKeyDto[]): Promise<CompareDto> {
  if (faces.length < COMPARE_MIN || faces.length > COMPARE_MAX) throw new Error(`compare needs ${COMPARE_MIN}–${COMPARE_MAX} faces, got ${faces.length}`);
  const settled = await Promise.allSettled(faces.map(f => api.getFace(f, getLocale())));
  const ok: FaceDetailDto[] = []; const failed: CompareDto['failed'] = [];
  settled.forEach((r, i) => { if (r.status === 'fulfilled') ok.push(r.value); else failed.push({ face: faces[i], reason: r.reason instanceof Error ? r.reason.message : String(r.reason) }); });
  if (ok.length < COMPARE_MIN) throw new Error(failed[0]?.reason ?? t('cmp.unreadable'));
  return buildCompare(ok, failed);
}
