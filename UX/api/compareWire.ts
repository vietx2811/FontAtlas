import type { CompareAxisDto } from '../bindings/CompareAxisDto';
import type { CompareDto as WireDto } from '../bindings/CompareDto';
import { COMPARE_METRICS, type CompareDto, type CompareFace, type CompareMetric } from './compare';

const quality = (a: CompareAxisDto): CompareMetric['quality'] => (a.estimated || a.quality === 'estimated' ? 'estimated' : a.quality === 'measured' ? 'measured' : 'failed');

function metric(key: string, a: CompareAxisDto | undefined, baseline: boolean): CompareMetric {
  if (!a || a.rawValue === null || !Number.isFinite(a.rawValue)) return { key, value: null, unit: a?.unit ?? null, quality: 'failed', reason: null, percentile: null, delta: null };
  return {
    key, value: a.rawValue, unit: a.unit, quality: quality(a), reason: null,
    percentile: Number.isFinite(a.percentile) ? a.percentile : null,
    delta: baseline ? null : { abs: a.delta ?? null, pct: a.deltaPct ?? null },
  };
}

/** Kết quả lệnh `compare_faces` → mô hình của màn Compare. Chịu được payload thưa (phân tích cũ). */
export function fromWire(dto: WireDto): CompareDto {
  const faces: CompareFace[] = dto.faces.map((f, i) => {
    const axes = f.axes ?? [];
    const v = f.vietnamese;
    return {
      face: f.face, row: f.row, classification: { ...f.classification, usage: f.classification?.usage ?? [], signals: f.classification?.signals ?? [] },
      metrics: Object.fromEntries(COMPARE_METRICS.map(k => [k, metric(k, axes.find(a => a.key === k), i === 0 || f.isBaseline)])),
      vi: {
        supports: v?.supports ?? false, missing: v?.missing ?? 0, total: v?.total ?? 0, stackedMarksOverflow: v?.overflow ?? false, stackedMarksTopEm: v?.stackedMarksTopEm ?? null,
        overAscenderPctEm: v?.overAscenderPctEm ?? null, overCapPctEm: v?.overCapPctEm ?? null,
      },
      license: f.license?.kind ?? null, designer: null,
      extras: f.extras ?? null, family: f.family ?? null, licenseNote: f.license ?? null,
    };
  });
  return { faces, failed: [] };
}
