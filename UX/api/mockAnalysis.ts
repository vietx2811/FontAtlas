// Dữ liệu phân tích giả nhưng ổn định theo từng face (cùng face → cùng kết quả), cho dev và Vitest.
import type { FaceRow } from '../features/library-grid/types';
import type { AxisReadingDto, PairCandidateDto, PairLabelDto, PairOptsDto, SimilarHitDto, SimilarSource } from './types';

type PairSymbol = '●' | '◐' | '◆' | '○' | '◑';

/** Mock theo đúng keys/units Rust; số dưới đây chỉ là fixture QA, không phải phép đo font. */
export const MEASURED_AXES = ['x_height_em', 'x_height_cap', 'ascender_em', 'stem', 'darkness', 'contrast', 'stress_deg', 'aperture', 'aperture_e', 'width', 'mean_advance', 'width_cv', 'spacing', 'slant_deg', 'serif_foot', 'counter'] as const;
export const CATEGORICAL_AXES = ['terminal', 'a_storeys', 'g_storeys'] as const;
export const MOCK_UNITS: Record<string, string> = {
  x_height_em: 'em', cap_height_em: 'em', ascender_em: 'em', descender_em: 'em', x_height_cap: 'cap_height_ratio',
  stem: 'x_height_ratio', aperture_e: 'x_height_ratio', width: 'x_height_ratio', mean_advance: 'x_height_ratio', spacing: 'x_height_ratio',
  stress_deg: 'deg', slant_deg: 'deg', aperture: 'deg', darkness: 'area_ratio', counter: 'area_ratio',
  contrast: 'ratio', contrast_h: 'ratio', width_cv: 'ratio', serif_foot: 'ratio', terminal: 'ratio',
};
const rnd = (seed: number, k: number): number => ((seed * 9301 + k * 49297 + 233) % 233280) / 233280;
export function axesFor(row: FaceRow): AxisReadingDto[] {
  const keys = [...MEASURED_AXES, 'terminal', 'a_storeys', 'g_storeys', 'cap_height_em', 'descender_em', 'contrast_h'];
  return keys.map((key, i) => {
    const r = rnd(row.faceId, i + 10);
    const quality = key === 'stress_deg' || i % 6 === 5 ? 'estimated' : 'measured';
    const base = { key, reason: null, method: 'mock_fixture', unit: MOCK_UNITS[key] ?? null, quality, percentile: i < 16 && key !== 'serif_foot' ? r : null };
    if (i === 12 && row.faceId % 3 === 0) return { ...base, value: null, rawValue: null, quality: 'failed', reason: 'no_latin_lowercase', percentile: null };
    const value = key === 'x_height_em' ? 0.46 + 0.1 * r : key === 'cap_height_em' ? 0.7 : key === 'descender_em' ? -0.2 : MOCK_UNITS[key] === 'deg' ? 30 * r : 0.1 + r;
    if (key === 'a_storeys' || key === 'g_storeys') return { ...base, value: row.faceId % 2 ? '1' : '2', rawValue: row.faceId % 2 ? 1 : 2, percentile: null };
    return { ...base, value: key === 'serif_foot' ? 'sans' : value, rawValue: value };
  });
}

const VI_MISSING_POOL = [...'ẰẲẴẶẸẺẼỀỂỄỆỈỊỌỎỐỒỔỖỘỚỜỞỠỢỤỦỨỪỬỮỰỲỴỶỸ', ...'ằẳẵặẹẻẽềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷỹ'];
export const VI_TOTAL = 134; // 67 chữ thường + 67 chữ hoa có dấu hoặc đ
export const viMissingFor = (n: number): string[] => VI_MISSING_POOL.slice(0, n);

const SAME = ['x_height_em', 'width', 'stem', 'aperture', 'serif_foot', 'x_height_cap'];
const DIFFER = ['contrast', 'slant_deg', 'darkness', 'spacing', 'terminal', 'stress_deg'];
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
const PAIR_AXES = ['x_height', 'contrast', 'weight', 'serif'];

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

/** Combo giả cho (heading, body): nhãn ●◐◆○ ổn định theo cặp, không có điểm số. */
export function comboAssessment(heading: FaceRow, body: FaceRow, seed: number): PairCandidateDto {
  const sym = (j: number): PairSymbol => (['●', '◐', '○', '●', '◆'] as const)[(seed + j + body.faceId) % 5];
  const labels: PairLabelDto[] = PAIR_AXES.map((axis, j) => ({ axis, relation: RELATIONS[sym(j)], symbol: sym(j), delta: null, a: null, b: null, roleReversed: false }));
  const clashes = labels.filter(l => l.symbol === '◆').map(l => l.axis);
  return {
    face: { hashHex: body.hashHex, faceIndex: body.faceIndex }, row: body, gate: clashes.length ? 'experimental' : 'safe', labels, clashes,
    bodyWarnings: heading.viMissing > 0 ? ['no_vietnamese'] : [], headingOverflow: false, insufficientData: false, weightHint: null, block: null,
    sentences: [`${heading.family} làm tiêu đề, ${body.family} làm nội dung: ${RELATIONS[labels[0].symbol]} về chiều cao x.`],
    summary: `${body.family} đi cùng ${heading.family}: ${RELATIONS[labels[0].symbol]}, ${RELATIONS[labels[1].symbol]}.`,
  };
}
