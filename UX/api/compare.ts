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
  const settled = await Promise.allSettled(faces.map(f => api.getFace(f, 'vi')));
  const ok: FaceDetailDto[] = []; const failed: CompareDto['failed'] = [];
  settled.forEach((r, i) => { if (r.status === 'fulfilled') ok.push(r.value); else failed.push({ face: faces[i], reason: r.reason instanceof Error ? r.reason.message : String(r.reason) }); });
  if (ok.length < COMPARE_MIN) throw new Error(failed[0]?.reason ?? 'không đọc được font');
  return buildCompare(ok, failed);
}
