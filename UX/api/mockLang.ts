import type { FaceRowDto } from '../bindings';
import type { LangBadgeDto, LangCoverageDto, LangFilterDto, LangItemDto, LangListDto } from './types';
import { VI_TOTAL } from './mockAnalysis';

type Row = Pick<FaceRowDto, 'faceId' | 'viMissing'>;
type Level = 'full' | 'near' | 'partial' | 'low';

/** Ngôn ngữ giả cho mock/test: mã ISO 639-3, đủ để dựng LangPicker và chỉ báo trên thẻ. */
export const MOCK_LANGS: ReadonlyArray<{ code: string; bcp47: string; nameEn: string; autonym: string; script: string; total: number }> = [
  { code: 'vie', bcp47: 'vi', nameEn: 'Vietnamese', autonym: 'Tiếng Việt', script: 'Latn', total: VI_TOTAL },
  { code: 'fra', bcp47: 'fr', nameEn: 'French', autonym: 'français', script: 'Latn', total: 86 },
  { code: 'pol', bcp47: 'pl', nameEn: 'Polish', autonym: 'polski', script: 'Latn', total: 70 },
  { code: 'tur', bcp47: 'tr', nameEn: 'Turkish', autonym: 'Türkçe', script: 'Latn', total: 58 },
  { code: 'rus', bcp47: 'ru', nameEn: 'Russian', autonym: 'русский', script: 'Cyrl', total: 66 },
  { code: 'ell', bcp47: 'el', nameEn: 'Greek', autonym: 'Ελληνικά', script: 'Grek', total: 48 },
];

export function levelOf(row: Row, code: string): Level {
  if (code === 'vie') return row.viMissing === 0 ? 'full' : row.viMissing <= 2 ? 'near' : row.viMissing <= 10 ? 'partial' : 'low';
  const idx = MOCK_LANGS.findIndex(l => l.code === code);
  if (idx < 0) return 'low';
  return (['full', 'full', 'near', 'partial', 'low'] as const)[(row.faceId + idx) % 5];
}
const satisfies = (lv: Level, want: 'full' | 'near') => lv === 'full' || (want === 'near' && lv === 'near');
export const matchesLangs = (row: Row, langs: LangFilterDto[], mode: 'all' | 'any' = 'all'): boolean =>
  !langs.length || (mode === 'all' ? langs.every(l => satisfies(levelOf(row, l.code), l.level)) : langs.some(l => satisfies(levelOf(row, l.code), l.level)));

export function badgeOf(row: Row, code = 'vie'): LangBadgeDto | null {
  const lang = MOCK_LANGS.find(l => l.code === code);
  if (!lang) return null;
  const lv = levelOf(row, code);
  const have = lv === 'full' ? lang.total : lv === 'near' ? Math.ceil(lang.total * 0.93) : lv === 'partial' ? Math.ceil(lang.total * 0.8) : Math.ceil(lang.total * 0.4);
  return { code, bcp47: lang.bcp47, state: lv, have, total: lang.total };
}

export function listFor(rows: Row[]): LangListDto {
  const items: LangItemDto[] = MOCK_LANGS.map(l => {
    const lv = rows.map(r => levelOf(r, l.code));
    return { code: l.code, bcp47: l.bcp47, nameEn: l.nameEn, autonym: l.autonym, script: l.script, full: lv.filter(x => x === 'full').length, near: lv.filter(x => x === 'full' || x === 'near').length };
  }).filter(i => i.near > 0);
  return { dbVersion: 'hg-mock-1', complete: true, ready: { done: rows.length, total: rows.length }, items };
}

export function coverageFor(row: Row, code: string): LangCoverageDto {
  const b = badgeOf(row, code);
  const total = b?.total ?? 0; const have = b?.have ?? 0;
  const miss = total - have;
  return { code, level: levelOf(row, code), baseHave: have, baseTotal: total, marksHave: 0, marksTotal: 0, missing: Array.from({ length: Math.min(miss, 200) }, (_, i) => String.fromCodePoint(0x0100 + i)), missingMarks: [], truncated: miss > 200 };
}
