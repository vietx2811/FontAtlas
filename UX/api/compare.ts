import type { AxisReadingDto, ClassificationDto, FaceDetailDto, FaceKeyDto, FaceRowDto } from '../bindings';

/** Các số đo hiện trong Compare, theo thứ tự hàng. Khoá = `AxisReadingDto.key` canonical. */
export const COMPARE_METRICS = ['x_height_em', 'cap_height_em', 'contrast', 'contrast_h', 'slant_deg', 'stress_deg', 'aperture', 'serif_foot'] as const;
export type CompareMetricKey = typeof COMPARE_METRICS[number];

export type CompareMetric = {
  key: string; value: number | null; unit: string | null;
  quality: 'measured' | 'estimated' | 'failed'; reason: string | null; percentile: number | null;
};
export type CompareFace = {
  face: FaceKeyDto; row: FaceRowDto; classification: ClassificationDto;
  metrics: Record<string, CompareMetric>;
  vi: { supports: boolean; missing: number; total: number; stackedMarksOverflow: boolean; stackedMarksTopEm: number | null };
  license: string | null; designer: string | null;
};
export type CompareDto = { faces: CompareFace[] };
export const COMPARE_MIN = 2;
export const COMPARE_MAX = 8;

const asQuality = (q: string): CompareMetric['quality'] => (q === 'measured' || q === 'estimated' ? q : 'failed');
function metricOf(key: string, a: AxisReadingDto | undefined): CompareMetric {
  if (!a || a.rawValue === null || !Number.isFinite(a.rawValue)) {
    return { key, value: null, unit: a?.unit ?? null, quality: 'failed', reason: a?.reason ?? null, percentile: null };
  }
  return { key, value: a.rawValue, unit: a.unit, quality: asQuality(a.quality), reason: a.reason, percentile: a.percentile };
}

/** Ghép chi tiết từng face (cùng thứ tự) thành mô hình cho màn Compare. Chỉ dùng số đo thật; thiếu → `failed`, không điền số. */
export function buildCompare(details: FaceDetailDto[]): CompareDto {
  return {
    faces: details.map(d => ({
      face: { hashHex: d.row.hashHex, faceIndex: d.row.faceIndex }, row: d.row, classification: d.classificationInfo,
      metrics: Object.fromEntries(COMPARE_METRICS.map(k => [k, metricOf(k, d.axes.find(a => a.key === k))])),
      vi: { supports: d.vietnamese.supports, missing: d.vietnamese.missing.length, total: d.vietnamese.total, stackedMarksOverflow: d.vietnamese.stackedMarksOverflow, stackedMarksTopEm: d.vietnamese.stackedMarksTopEm },
      license: d.metadata.license, designer: d.metadata.designer,
    })),
  };
}

/** Triển khai chung (tauri + mock): đọc từng face bằng getFace rồi ghép. Một face lỗi → cả lệnh lỗi, không có cột thiếu. */
export async function composeCompare(api: { getFace(f: FaceKeyDto, lang: 'vi' | 'en'): Promise<FaceDetailDto> }, faces: FaceKeyDto[]): Promise<CompareDto> {
  if (faces.length < COMPARE_MIN || faces.length > COMPARE_MAX) throw new Error(`compare needs ${COMPARE_MIN}–${COMPARE_MAX} faces, got ${faces.length}`);
  return buildCompare(await Promise.all(faces.map(f => api.getFace(f, 'vi'))));
}
