import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, AxisStatsDto } from '../../api';
import { toGridQuery, useUiStore } from '../../store/ui';
import { useScanStore } from '../../store/scan';
import { AXIS_META, numberText, roundedValue, unitText } from '../detail/tabs/measurements';
import './filters.css';

// Intent: chọn miền số đo thật trong thư viện, thao tác gọn bên cạnh tìm kiếm.
// Palette/depth/surfaces/type: token card/border/fg/accent và Inter hiện có. Spacing: 4/8/12/16.
export function FilterPanel({ api }: { api: Api }): React.JSX.Element {
  const state = useUiStore();
  const { filters } = state;
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const [stats, setStats] = useState<AxisStatsDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const version = useScanStore(s => s.version);
  const { rawAxisRanges: _ranges, ...metadata } = toGridQuery(state);
  const metadataKey = JSON.stringify(metadata);
  const statsQuery = useMemo(() => JSON.parse(metadataKey) as typeof metadata, [metadataKey]);
  useEffect(() => {
    if (!open) return;
    let alive = true;
    setStats(null); setError(null);
    api.axisStats(statsQuery).then(s => { if (alive) setStats(s); }, e => { if (alive) setError(e instanceof Error ? e.message : String(e)); });
    return () => { alive = false; };
  }, [api, open, statsQuery, version, retry]);
  useEffect(() => {
    if (!open) return;
    dialog.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const close = (e: PointerEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false); };
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); setOpen(false); trigger.current?.focus(); } };
    document.addEventListener('pointerdown', close);
    document.addEventListener('keydown', key, true);
    return () => { document.removeEventListener('pointerdown', close); document.removeEventListener('keydown', key, true); };
  }, [open]);
  const ranges = filters.rawAxisRanges ?? [];
  const count = ranges.length + Number(filters.variable != null) + Number(filters.italic != null);
  const update = (axis: string, range: [number, number] | null) => {
    const others = (useUiStore.getState().filters.rawAxisRanges ?? []).filter(r => r.axis !== axis);
    state.setFilters({ ...useUiStore.getState().filters, rawAxisRanges: range ? [...others, { axis, range }] : others });
  };
  return <div className="fa-filter-root" ref={root}>
    <button ref={trigger} type="button" className="fa-vi-toggle" aria-expanded={open} aria-controls="font-filters" aria-pressed={count > 0} onClick={() => setOpen(v => !v)}>Lọc{count ? ` · ${count}` : ''}</button>
    {open && <div ref={dialog} id="font-filters" className="fa-filters" role="dialog" aria-label="Bộ lọc font">
      <header><strong>Bộ lọc</strong><button type="button" aria-label="Đóng bộ lọc" onClick={() => { setOpen(false); trigger.current?.focus(); }}>×</button></header>
      <div className="fa-filter-content">
        <fieldset><legend>Loại font</legend>
          <label><input type="checkbox" checked={filters.variable === true} onChange={e => state.setFilters({ ...filters, variable: e.target.checked ? true : undefined })} />Variable</label>
          <label><input type="checkbox" checked={filters.italic === true} onChange={e => state.setFilters({ ...filters, italic: e.target.checked ? true : undefined })} />In nghiêng</label>
        </fieldset>
        <h3>Số đo</h3>
        <p className="fa-filter-help">Chỉ font có số đo phù hợp. Các điều kiện kết hợp cùng nhau.</p>
        {error ? <p role="alert">Không tải được miền số đo: {error} <button type="button" onClick={() => setRetry(n => n + 1)}>Thử lại</button></p>
          : stats === null ? <p role="status">Đang đọc thư viện…</p>
          : stats.map(s => <AxisRange key={s.axis} stats={s} selected={ranges.find(r => r.axis === s.axis)?.range} onChange={range => update(s.axis, range)} />)}
      </div>
      <footer><span>{count} điều kiện</span><button type="button" disabled={!count} onClick={() => state.setFilters({})}>Đặt lại</button></footer>
    </div>}
  </div>;
}

function AxisRange({ stats: s, selected, onChange }: { stats: AxisStatsDto; selected?: [number, number]; onChange(range: [number, number] | null): void }): React.JSX.Element {
  const label = AXIS_META[s.axis]?.label ?? s.axis;
  const empty = s.count === 0 || s.min === null || s.max === null;
  const constant = !empty && s.min === s.max;
  const [full, setFull] = useState(false);
  const min = roundedValue(s.min ?? 0, s.unit, 'down');
  const max = roundedValue(s.max ?? 0, s.unit, 'up');
  const focusMin = roundedValue(s.p1 ?? s.min ?? 0, s.unit, 'down');
  const focusMax = roundedValue(s.p99 ?? s.max ?? 0, s.unit, 'up');
  const range = selected ?? (constant ? [min, max] : [roundedValue(s.p1 ?? s.min ?? 0, s.unit), roundedValue(s.p99 ?? s.max ?? 0, s.unit)]);
  const [draft, setDraft] = useState(() => range.map(v => numberText(v, s.unit)));
  const [error, setError] = useState<string | null>(null);
  const rangeKey = JSON.stringify(range);
  useEffect(() => { setDraft(range.map(v => numberText(v, s.unit))); setError(null); }, [rangeKey, s.unit]);
  const outlier = !!selected && (selected[0] < focusMin || selected[1] > focusMax);
  const lo = full || outlier ? min : focusMin;
  const hi = full || outlier ? max : focusMax;
  const commit = () => {
    const [a, b] = draft.map(v => v.trim() ? roundedValue(Number(v.replace(',', '.')), s.unit) : NaN);
    if (!Number.isFinite(a) || !Number.isFinite(b) || a > b || a < min || b > max) {
      setError('Nhập khoảng hợp lệ trong min–max.'); return;
    }
    setError(null); onChange([a, b]);
  };
  return <fieldset className="fa-axis-filter">
    <legend><label><input type="checkbox" checked={!!selected} disabled={empty} onChange={e => onChange(e.target.checked ? [range[0], range[1]] : null)} />{label}</label></legend>
    <div className="fa-axis-domain"><span>{unitText(s.unit)} · {s.count} font</span>{!empty && !constant && <button type="button" aria-pressed={full || outlier} disabled={outlier} title={outlier ? 'Khoảng đang chọn có ngoại lệ ngoài P1–P99' : undefined} onClick={() => { if (!full) { setFull(true); if (selected) onChange([min, max]); } else setFull(false); }}>{outlier ? 'Đang dùng min–max' : full ? 'Khoảng P1–P99' : 'Toàn khoảng'}</button>}</div>
    {empty ? <p>Chưa có số đo</p> : <>
      <div className="fa-range-inputs">{[0, 1].map(i => <label key={i}><span>{i === 0 ? 'Từ' : 'Đến'}</span><input type="text" inputMode="decimal" aria-label={`${label} ${i === 0 ? 'từ' : 'đến'}`} disabled={!selected} value={draft[i]} onChange={e => { const next = [...draft]; next[i] = e.target.value; setDraft(next); }} onBlur={commit} onKeyDown={e => { if (e.key === 'Enter') commit(); }} /></label>)}</div>
      <div className="fa-range-sliders">{[0, 1].map(i => <input key={i} type="range" aria-label={`${label} ${i === 0 ? 'cận dưới' : 'cận trên'}`} disabled={!selected || constant} min={lo} max={hi} step="any" value={range[i]} onChange={e => { const n = roundedValue(Number(e.target.value), s.unit); onChange(i === 0 ? [Math.min(n, range[1]), range[1]] : [range[0], Math.max(n, range[0])]); }} />)}</div>
      <p>{constant ? `Cùng giá trị ${numberText(s.min!, s.unit)}; thanh trượt không cần chỉnh.` : `Min ${numberText(min, s.unit)} · max ${numberText(max, s.unit)}`}</p>
      {error && <p role="alert">{error}</p>}
    </>}
  </fieldset>;
}
