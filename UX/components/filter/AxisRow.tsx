import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { RangeSlider, type Range } from './RangeSlider';
import { TermTip, termKey, termName } from '../ui/terms';
import './axisRow.css';

export type AxisRowProps = {
  /** Khoá backend của trục (`x_height_em`, `weight_class`…): quyết định tên, gloss và hình của TermTip. */
  axis: string;
  /** Ghi chú nguồn số liệu cạnh tên, ví dụ "measured" hay "OS/2 class". */
  source?: string;
  min: number; max: number; step: number; ticks?: number[];
  value: Range | null;
  digits?: number;
  /** Chữ hiển thị cho một giá trị (vd 700 → "Bold"); mặc định là số. */
  format?(v: number): string;
  onChange(next: Range | null): void;
  /** Lớp vẽ sau thanh: histogram. */
  children?: ReactNode;
  /** Khu phải của hàng (nút wght/wdth…). */
  extra?: ReactNode;
  empty?: boolean;
  count?: number;
};

const parse = (s: string): number => Number(s.trim().replace(',', '.'));

/** Một hàng bộ lọc trục: tên (TermTip có hình), slider hai đầu, ô nhập số tại chỗ. Nháy đúp thanh = bỏ lọc. */
export function AxisRow({ axis, source, min, max, step, ticks, value, digits = 2, format, onChange, children, extra, empty, count }: AxisRowProps): React.JSX.Element {
  const fmt = format ?? ((v: number) => new Intl.NumberFormat('vi-VN', { useGrouping: false, maximumFractionDigits: digits }).format(v));
  const shown: Range = value ?? [min, max];
  // Ô nhập hiện số làm tròn `digits` chữ số (miền đo có thể dài 10 chữ số); chỉ ghi lại khi người dùng thật sự sửa chữ.
  const seed = (r: Range): [string, string] => [String(Number(r[0].toFixed(digits))), String(Number(r[1].toFixed(digits)))];
  const [draft, setDraft] = useState<[string, string]>(() => seed(shown));
  const [bad, setBad] = useState(false);
  useEffect(() => { setDraft(seed(shown)); setBad(false); }, [shown[0], shown[1]]); // eslint-disable-line react-hooks/exhaustive-deps
  const name = termName(axis);
  const commit = () => {
    const base = seed(shown);
    if (draft[0] === base[0] && draft[1] === base[1]) { setBad(false); return; }
    // Đầu nào chưa sửa thì giữ nguyên giá trị đầy đủ (không bị làm tròn ngầm).
    const tol = 10 ** -digits / 2;
    // Số gõ khớp mép miền (sau làm tròn hiển thị) thì coi là đúng mép.
    const edge = (v: number) => Math.abs(v - min) <= tol ? min : Math.abs(v - max) <= tol ? max : v;
    const a = draft[0] === base[0] ? shown[0] : edge(parse(draft[0])); const b = draft[1] === base[1] ? shown[1] : edge(parse(draft[1]));
    if (!Number.isFinite(a) || !Number.isFinite(b) || a > b || a < min || b > max) { setBad(true); return; }
    setBad(false);
    if (a === shown[0] && b === shown[1]) return;
    onChange(a <= min && b >= max ? null : [a, b]);
  };
  return (
    <div className="fa-axrow" data-testid="axis-filter-row" data-axis={axis} data-active={value !== null} data-empty={!!empty}>
      <div className="fa-axrow-head">
        <TermTip term={termKey(axis)}><span className="fa-axis-name" data-term={termKey(axis)}>{name}</span></TermTip>
        {source && <span className="fa-axrow-src">{source}</span>}
        {count !== undefined && <span className="fa-axrow-count">{count}</span>}
        <span className="fa-axrow-extra">{extra}</span>
      </div>
      {empty
        ? <p className="fa-axrow-empty">—</p>
        : <>
          <RangeSlider min={min} max={max} step={step} ticks={ticks} value={value} label={name} format={fmt} onChange={r => onChange(r[0] <= min && r[1] >= max ? null : r)} onReset={() => onChange(null)}>{children}</RangeSlider>
          <div className="fa-axrow-nums">
            {[0, 1].map(i => (
              <input key={i} type="text" inputMode="decimal" aria-label={`${name} ${i === 0 ? '↓' : '↑'}`} aria-invalid={bad} value={draft[i]}
                onChange={e => setDraft(d => (i === 0 ? [e.target.value, d[1]] : [d[0], e.target.value]))}
                onBlur={commit} onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); commit(); } }} />
            ))}
            {format && value && <span className="fa-axrow-words">{fmt(value[0])}–{fmt(value[1])}</span>}
          </div>
        </>}
    </div>
  );
}
