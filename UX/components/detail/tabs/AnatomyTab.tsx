import type { CSSProperties } from 'react';
import type { AxisReadingDto, FaceDetailDto } from '../../../api';
import { t } from '../../../i18n';
import { useGuides } from '../../ui/inkLines';
import { AxisIcon } from './axisIcons';
import { TermTip, termKey } from '../../ui/terms';

import { AXIS_META, numberText, unitText } from './measurements';
import { Tooltip } from '../../ui/Tooltip';
import { NoteMark } from '../../ui/noteMark';
export { AXIS_META } from './measurements';

const CATEGORY_TEXT: Record<string, Record<string, string>> = {
  a_storeys: { '1': 'a một tầng', '2': 'a hai tầng' }, g_storeys: { '1': 'g một tầng', '2': 'g hai tầng' },
  serif_foot: { sans: 'không chân', serif: 'có chân', uncertain: 'chưa rõ phân loại' },
};
const REASONS: Record<string, string> = {
  no_latin_lowercase: 'thiếu chữ thường Latin', missing_glyph: 'thiếu glyph cần đo', no_outlines: 'không có đường nét', low_contrast: 'tương phản quá thấp để đo', degenerate: 'đường nét không đủ để đo',
};

export function percentileWord(p: number): string {
  return p < .2 ? 'rất thấp' : p < .4 ? 'thấp' : p < .6 ? 'trung bình' : p < .8 ? 'cao' : 'rất cao';
}

export type GuideLine = { id: 'cap' | 'x' | 'baseline'; y: number };
/** Đường dẫn của glyph Hxg: chỉ vẽ đường có số đo thật (không đoán). `size` = cỡ chữ SVG, `baseline` = toạ độ y của đường nền. */
export function glyphLines(axes: AxisReadingDto[], size: number, baseline: number): GuideLine[] {
  const v = (key: string) => { const a = axes.find(x => x.key === key); return a && a.quality !== 'failed' && typeof a.rawValue === 'number' && Number.isFinite(a.rawValue) ? a.rawValue : null; };
  const out: GuideLine[] = [];
  const cap = v('cap_height_em'); const x = v('x_height_em');
  if (cap !== null) out.push({ id: 'cap', y: baseline - cap * size });
  if (x !== null) out.push({ id: 'x', y: baseline - x * size });
  out.push({ id: 'baseline', y: baseline });
  return out;
}
const LINE_LABEL = { cap: 'cap', x: 'x', baseline: 'nền' } as const;

function AxisRow({ a }: { a: AxisReadingDto }): React.JSX.Element {
  const meta = AXIS_META[a.key] ?? { label: a.key };
  const failed = a.quality === 'failed';
  const numeric = !failed && !['a_storeys', 'g_storeys'].includes(a.key) && a.rawValue !== null && Number.isFinite(a.rawValue);
  const category = CATEGORY_TEXT[a.key]?.[String(a.value)];
  const estimated = a.quality === 'estimated';
  const missing = !numeric && !category;
  const noRaw = a.key === 'serif_foot' && !numeric && !failed;
  const pct = !failed && !missing && a.percentile !== null ? `P${Math.round(a.percentile * 100)} · ${percentileWord(a.percentile)}` : null;
  const noteText = failed ? REASONS[a.reason ?? ''] ?? a.reason ?? t('anat.reasonUnknown')
    : [missing ? t('anat.noData') : noRaw ? t('anat.noRaw') : estimated ? t('anat.estimated') : category && numeric ? category : null, pct && `${pct} (${t('anat.percentileTip')})`].filter(Boolean).join(' · ');
  return (
    <Tooltip label={[a.method, a.rawValue === null ? null : t('anat.rawValue', { v: a.rawValue })].filter(Boolean).join(' · ')}>
    <li className="fa-axis" data-testid="axis-row" data-axis={a.key} data-quality={a.quality}>
      <span className="fa-axis-ico"><AxisIcon name={a.key} /></span>
      <TermTip term={a.key}><span className="fa-axis-name" data-term={termKey(a.key)}>{meta.label}</span></TermTip>
      <span className="fa-axis-value">{failed ? t('anat.failed') : numeric ? <>{numberText(a.rawValue!, a.unit)} <small>{unitText(a.unit)}</small></> : category ?? t('anat.unmeasured')}</span>
      {noteText && <NoteMark inline kind={failed ? 'warn' : 'info'} text={noteText} />}
    </li>
    </Tooltip>
  );
}

const axisEm = (axes: AxisReadingDto[], key: string): number | null => { const a = axes.find(x => x.key === key); return a && a.quality !== 'failed' && typeof a.rawValue === 'number' && Number.isFinite(a.rawValue) ? a.rawValue : null; };

/** Đường kẻ theo mực thật của chữ đang vẽ (không theo số analyzer); chỉ vẽ đường nào đo được. Baseline luôn là nền của chữ. */
export function guideLinesFrom(g: { cap: number | null; x: number | null }, size: number, baseline: number): GuideLine[] {
  const out: GuideLine[] = [];
  if (g.cap !== null) out.push({ id: 'cap', y: baseline - g.cap * size });
  if (g.x !== null) out.push({ id: 'x', y: baseline - g.x * size });
  out.push({ id: 'baseline', y: baseline });
  return out;
}
const f3 = (v: number | null) => (v === null ? '—' : numberText(v, 'em'));

/** Hxg vẽ bằng ĐÚNG face đã đo (family của face đó, không weight/italic của style chọn, không tổng hợp đậm/nghiêng). */
export const hxgStyle = (family: string | null): CSSProperties => ({
  fontFamily: family ? `"${family}"` : undefined, fontWeight: 'normal', fontStyle: 'normal', fontSynthesis: 'none', fontVariationSettings: 'normal', opacity: family ? 1 : 0.3,
});

export function AnatomyTab({ detail, family }: { detail: FaceDetailDto; family: string | null }): React.JSX.Element {
  const SIZE = 100; const BASE = 112;
  const analyzer = { cap: axisEm(detail.axes, 'cap_height_em'), x: axisEm(detail.axes, 'x_height_em') };
  const g = useGuides(family, analyzer, detail.row.faceIndex);
  const lines = g.status === 'ok' || g.status === 'mismatch' ? guideLinesFrom(g, SIZE, BASE) : [];
  return (
    <>
      <div className="fa-note-host fa-hxg-host">
      <svg className="fa-hxg" role="img" aria-label={t('anat.hxgAria')} viewBox="0 0 300 150" width="100%" data-guides={g.status}>
        {lines.map(l => (
          <g key={l.id}>
            <line data-testid="guide-line" data-line={l.id} x1="6" x2="294" y1={l.y} y2={l.y} stroke="var(--accent)" strokeWidth="0.8" strokeDasharray={l.id === 'baseline' ? undefined : '3 3'} />
            <text x="294" y={l.y - 3} textAnchor="end" fontSize="8" fill="var(--muted)" fontFamily="var(--font-ui)">{LINE_LABEL[l.id]}</text>
          </g>
        ))}
        <text data-testid="hxg-text" x="14" y={BASE} fontSize={SIZE} fill="var(--fg)" style={hxgStyle(family)}>Hxg</text>
      </svg>
      {g.status === 'unavailable' && <NoteMark testId="guides-note" text={t('anat.guides.unavailable')} kind="info" />}
      {g.status === 'mismatch' && <NoteMark testId="guides-note" text={`${t('anat.guides.mismatch')} — ${t('anat.guides.mismatchTip', { capA: f3(g.analyzer.cap), xA: f3(g.analyzer.x), capI: f3(g.ink.cap), xI: f3(g.ink.x) })}`} />}
      </div>
      <ul className="fa-axes">{detail.axes.map(a => <AxisRow key={a.key} a={a} />)}</ul>
    </>
  );
}
