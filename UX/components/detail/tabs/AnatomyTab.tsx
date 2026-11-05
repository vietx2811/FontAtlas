import type { CSSProperties } from 'react';
import type { AxisReadingDto, FaceDetailDto } from '../../../api';
import { AxisIcon } from './axisIcons';

export const AXIS_META: Record<string, { label: string; hint?: string }> = {
  x_height: { label: 'Chiều cao x' }, cap_height: { label: 'Chiều cao chữ hoa' }, ascender: { label: 'Nét lên (ascender)' }, descender: { label: 'Nét xuống (descender)' },
  stem_weight: { label: 'Độ đậm nét' }, contrast: { label: 'Tương phản nét' }, stress_deg: { label: 'Trục nhấn' }, slant_deg: { label: 'Độ nghiêng' },
  avg_width: { label: 'Độ rộng chữ' }, aperture: { label: 'Độ mở' }, counter_ratio: { label: 'Lòng chữ' }, letterspacing: { label: 'Khoảng cách chữ' },
  serif_foot: { label: 'Chân serif' }, arm_length: { label: 'Tay chữ' }, tail_length: { label: 'Đuôi chữ' }, roundness: { label: 'Độ tròn' },
  terminal: { label: 'Đầu nét' }, a_storeys: { label: 'Chữ a' }, g_storeys: { label: 'Chữ g' },
};
const CATEGORY_TEXT: Record<string, Record<string, string>> = {
  a_storeys: { '1': 'a một tầng', '2': 'a hai tầng' }, g_storeys: { '1': 'g một tầng', '2': 'g hai tầng' },
  terminal: { flat: 'đầu nét phẳng', round: 'đầu nét tròn', sheared: 'đầu nét vát' },
};
const REASONS: Record<string, string> = {
  no_latin_lowercase: 'thiếu chữ thường Latin', missing_glyph: 'thiếu glyph cần đo', no_outlines: 'không có đường nét',
};

export function percentileWord(p: number): string {
  return p < 20 ? 'rất thấp' : p < 40 ? 'thấp' : p < 60 ? 'trung bình' : p < 80 ? 'cao' : 'rất cao';
}

export type GuideLine = { id: 'cap' | 'x' | 'baseline'; y: number };
/** Đường dẫn của glyph Hxg: chỉ vẽ đường có số đo thật (không đoán). `size` = cỡ chữ SVG, `baseline` = toạ độ y của đường nền. */
export function glyphLines(axes: AxisReadingDto[], size: number, baseline: number): GuideLine[] {
  const v = (key: string) => { const a = axes.find(x => x.key === key); return a && a.quality !== 'failed' && typeof a.value === 'number' ? a.value : null; };
  const out: GuideLine[] = [];
  const cap = v('cap_height'); const x = v('x_height');
  if (cap !== null) out.push({ id: 'cap', y: baseline - cap * size });
  if (x !== null) out.push({ id: 'x', y: baseline - x * size });
  out.push({ id: 'baseline', y: baseline });
  return out;
}
const LINE_LABEL = { cap: 'cap', x: 'x', baseline: 'nền' } as const;

function AxisRow({ a }: { a: AxisReadingDto }): React.JSX.Element {
  const meta = AXIS_META[a.key] ?? { label: a.key };
  const cat = CATEGORY_TEXT[a.key];
  const dashed = a.quality === 'estimated';
  return (
    <li className="fa-axis" data-testid="axis-row" data-axis={a.key} data-quality={a.quality}>
      <span className="fa-axis-ico"><AxisIcon name={a.key} /></span>
      <span className="fa-axis-name">{meta.label}</span>
      {a.quality === 'failed' ? (
        <span className="fa-axis-note wide">Không đo được: {REASONS[a.reason ?? ''] ?? a.reason ?? 'không rõ lý do'}</span>
      ) : cat ? (
        <span className="fa-axis-note wide">{cat[String(a.value)] ?? String(a.value)}{dashed && <> · <em>ước lượng</em></>}</span>
      ) : (
        <>
          <span className={`fa-axis-track${dashed ? ' dashed' : ''}`} data-track>
            {a.percentile !== null && <i data-marker style={{ left: `${a.percentile}%` }} />}
          </span>
          <span className="fa-axis-note">{a.percentile !== null ? percentileWord(a.percentile) : ''}{dashed && <> · <em>ước lượng</em></>}</span>
        </>
      )}
    </li>
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
