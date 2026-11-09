import type { CSSProperties } from 'react';
import type { AxisReadingDto, FaceDetailDto } from '../../../api';
import { AxisIcon } from './axisIcons';
import { TermTip, termKey } from '../../ui/terms';

import { AXIS_META, numberText, unitText } from './measurements';
import { Tooltip } from '../../ui/Tooltip';
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
  return (
    <Tooltip label={[a.method, a.rawValue === null ? null : `Giá trị gốc: ${a.rawValue}`].filter(Boolean).join(' · ')}>
    <li className="fa-axis" data-testid="axis-row" data-axis={a.key} data-quality={a.quality}>
      <span className="fa-axis-ico"><AxisIcon name={a.key} /></span>
      <TermTip term={a.key}><span className="fa-axis-name" data-term={termKey(a.key)}>{meta.label}</span></TermTip>
      <span className="fa-axis-value">{failed ? 'Không đo được' : numeric ? <>{numberText(a.rawValue!, a.unit)} <small>{unitText(a.unit)}</small></> : category ?? 'Chưa đo'}</span>
      {(failed || missing || noRaw || estimated || category && numeric || a.percentile !== null) && <span className="fa-axis-note">
        {failed ? REASONS[a.reason ?? ''] ?? a.reason ?? 'không rõ lý do' : <>
          {missing ? 'Chưa có dữ liệu' : noRaw ? 'Chưa có số đo' : estimated ? 'Ước lượng' : category && numeric ? category : ''}
          {!missing && a.percentile !== null && <Tooltip label="Vị trí so với tập tham chiếu; không phải độ tin cậy"><span>{estimated ? ' · ' : ''}P{Math.round(a.percentile * 100)} · {percentileWord(a.percentile)}</span></Tooltip>}
        </>}
      </span>}
    </li>
    </Tooltip>
  );
}

export function AnatomyTab({ detail, css }: { detail: FaceDetailDto; css: CSSProperties }): React.JSX.Element {
  const SIZE = 100; const BASE = 112;
  const lines = glyphLines(detail.axes, SIZE, BASE);
  return (
    <>
      <svg className="fa-hxg" role="img" aria-label="Glyph Hxg với đường chữ hoa, chiều cao x và đường nền" viewBox="0 0 300 150" width="100%">
        {lines.map(l => (
          <g key={l.id}>
            <line data-testid="guide-line" data-line={l.id} x1="6" x2="294" y1={l.y} y2={l.y} stroke="var(--accent)" strokeWidth="0.8" strokeDasharray={l.id === 'baseline' ? undefined : '3 3'} />
            <text x="294" y={l.y - 3} textAnchor="end" fontSize="8" fill="var(--muted)" fontFamily="var(--font-ui)">{LINE_LABEL[l.id]}</text>
          </g>
        ))}
        <text x="14" y={BASE} fontSize={SIZE} fill="var(--fg)" style={css}>Hxg</text>
      </svg>
      <ul className="fa-axes">{detail.axes.map(a => <AxisRow key={a.key} a={a} />)}</ul>
    </>
  );
}
