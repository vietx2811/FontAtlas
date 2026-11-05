// Dữ liệu phân tích giả nhưng ổn định theo từng face (cùng face → cùng kết quả), cho dev và Vitest.
import type { FaceRow } from '../features/library-grid/types';
import type { AxisReadingDto, PairCandidateDto, PairLabelDto, PairOptsDto, SimilarHitDto, SimilarSource } from './types';

type PairSymbol = '●' | '◐' | '◆' | '○' | '◑';

/** 16 trục đo + terminal + hai trục phân loại (a_storeys, g_storeys). */
export const MEASURED_AXES = [
  'x_height', 'cap_height', 'ascender', 'descender', 'stem_weight', 'contrast', 'stress_deg', 'slant_deg',
  'avg_width', 'aperture', 'counter_ratio', 'letterspacing', 'serif_foot', 'arm_length', 'tail_length', 'roundness',
] as const;
export const CATEGORICAL_AXES = ['terminal', 'a_storeys', 'g_storeys'] as const;

const rnd = (seed: number, k: number): number => ((seed * 9301 + k * 49297 + 233) % 233280) / 233280;
const KEEP_FOR_LINES: Record<string, (s: number) => number> = {
  x_height: s => 0.46 + 0.1 * rnd(s, 1), cap_height: s => 0.66 + 0.08 * rnd(s, 2), ascender: s => 0.74 + 0.08 * rnd(s, 3), descender: s => 0.2 + 0.06 * rnd(s, 4),
};

export function axesFor(row: FaceRow): AxisReadingDto[] {
  const out: AxisReadingDto[] = MEASURED_AXES.map((key, i) => {
    const r = rnd(row.faceId, i + 10);
    const base = { key, reason: null as string | null, method: 'outline' as string | null };
    if (i === 14 && row.faceId % 3 === 0) return { ...base, value: null, quality: 'failed', reason: 'no_latin_lowercase', method: null, percentile: null };
    const quality = i % 6 === 5 ? 'estimated' : 'measured';
    const value = KEEP_FOR_LINES[key] ? KEEP_FOR_LINES[key](row.faceId) : Math.round(r * 1000) / 1000;
    return { ...base, value, quality, method: quality === 'estimated' ? 'stem_sampling' : 'outline', percentile: Math.round(r * 100) };
  });
  out.push(
    { key: 'terminal', value: row.faceId % 2 ? 'flat' : 'round', quality: 'measured', reason: null, method: 'outline', percentile: null },
    { key: 'a_storeys', value: row.faceId % 4 === 0 ? '1' : '2', quality: 'measured', reason: null, method: 'outline', percentile: null },
    { key: 'g_storeys', value: row.faceId % 5 === 0 ? '2' : '1', quality: 'estimated', reason: null, method: 'glyph_class', percentile: null },
  );
  return out;
}

const VI_MISSING_POOL = [...'ẰẲẴẶẸẺẼỀỂỄỆỈỊỌỎỐỒỔỖỘỚỜỞỠỢỤỦỨỪỬỮỰỲỴỶỸ', ...'ằẳẵặẹẻẽềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷỹ'];
export const VI_TOTAL = 134; // 67 chữ thường + 67 chữ hoa có dấu hoặc đ
export const viMissingFor = (n: number): string[] => VI_MISSING_POOL.slice(0, n);

const SAME = ['x_height', 'avg_width', 'stem_weight', 'aperture', 'serif_foot', 'cap_height'];
const DIFFER = ['contrast', 'slant_deg', 'roundness', 'letterspacing', 'terminal', 'stress_deg'];
const pick = (list: string[], seed: number, n: number): string[] => Array.from({ length: n }, (_, i) => list[(seed + i * 2) % list.length]);

export function similarFor(row: FaceRow, rows: FaceRow[], k: number): SimilarHitDto[] {
  const others = rows.filter(r => r.family !== row.family);
  const start = others.findIndex(r => r.faceId > row.faceId);
  const ordered = [...others.slice(Math.max(0, start)), ...others.slice(0, Math.max(0, start))];
  return ordered.slice(0, k).map((r, i) => {
    const source: SimilarSource = r.faceId % 3 === 0 ? 'axes_and_image' : 'axes';
    return {
      face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, row: r, distance: 0.1 + i * 0.07, source,
      same: pick(SAME, r.faceId, 2), differ: pick(DIFFER, r.faceId + i, 1),
    };
  });
}

const RELATIONS: Record<PairSymbol, string> = { '●': 'hoà hợp', '◐': 'tương phản', '◆': 'va chạm', '○': 'chưa chắc', '◑': 'gợi ý độ đậm' };
const PAIR_AXES = ['x_height', 'contrast', 'stem_weight', 'serif_foot'];

export function pairFor(row: FaceRow, rows: FaceRow[], o: PairOptsDto): PairCandidateDto[] {
  if (o.projectVi && row.viMissing > 0) return [];
  const mode = o.mode ?? 'safe';
  const pool = rows.filter(r => r.family !== row.family && !r.isItalic && (r.weightClass ?? 400) <= 500 && (!o.projectVi || r.viMissing === 0));
  const seen = new Set<string>();
  const uniq = pool.filter(r => (seen.has(r.family) ? false : (seen.add(r.family), true)));
  return uniq.slice(0, o.k).map((r, i) => {
    const sym = (j: number): PairSymbol => {
      if (mode === 'experimental' && j === 0 && i % 2 === 0) return '◆';
      if (mode !== 'safe' && j === 1) return i % 2 ? '◑' : '◐';
      return (['●', '◐', '○', '●'] as const)[(i + j) % 4];
    };
    const labels: PairLabelDto[] = PAIR_AXES.map((axis, j) => ({ axis, relation: RELATIONS[sym(j)], symbol: sym(j), delta: null, a: null, b: null, roleReversed: false }));
    return {
      face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, row: r, gate: 'passed', labels,
      clashes: labels.filter(l => l.symbol === '◆').map(l => l.axis), bodyWarnings: [], headingOverflow: false, insufficientData: false,
      weightHint: labels.some(l => l.symbol === '◑') ? 'Tăng độ đậm nội dung một nấc' : null, block: null,
      sentences: [`${row.family} làm tiêu đề, ${r.family} làm nội dung: ${RELATIONS[labels[0].symbol]} về chiều cao x.`],
      summary: `${r.family} đi cùng ${row.family}: ${RELATIONS[labels[0].symbol]}, ${RELATIONS[labels[1].symbol]}.`,
    };
  });
}
