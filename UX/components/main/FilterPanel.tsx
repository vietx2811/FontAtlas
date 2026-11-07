import { useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import type { Api, AxisStatsDto } from '../../api';
import { toMetadataQuery, useUiStore, type LibraryFilters } from '../../store/ui';
import { useScanStore } from '../../store/scan';
import { AXIS_META, numberText, roundedValue, unitText } from '../detail/tabs/measurements';
import './filters.css';
import { Tooltip } from '../ui/Tooltip';
import { useSources } from './FilterChips';
import { LICENSE_OPTIONS, VI_OPTIONS, forcedFacets } from './filterOptions';
import { t } from '../../i18n';
import { HIDDEN_CLASSES, classGloss, classLabel, subclassLabel } from './classLabels';
import type { ClassFacetDto } from '../../api';

// Intent: chọn miền số đo thật trong thư viện, thao tác gọn bên cạnh tìm kiếm.
// Palette/depth/surfaces/type: token card/border/fg/accent và Inter hiện có. Spacing: 4/8/12/16.
export function FilterPanel({ api }: { api: Api }): React.JSX.Element {
  const state = useUiStore(useShallow(s => ({ view: s.view, tag: s.tag, query: s.query, viOnly: s.viOnly, filters: s.filters, setFilters: s.setFilters })));
  const { filters } = state;
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const [stats, setStats] = useState<AxisStatsDto[] | null>(null);
  const [facets, setFacets] = useState<ClassFacetDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const version = useScanStore(s => s.version);
  const { rawAxisRanges: _ranges, ...metadata } = toMetadataQuery(state);
  const metadataKey = JSON.stringify(metadata);
  const statsQuery = useMemo(() => JSON.parse(metadataKey) as typeof metadata, [metadataKey]);
  useEffect(() => {
    if (!open) return;
    let alive = true;
    setLoading(true); setError(null);
    api.classFacets(statsQuery).then(f => { if (alive) setFacets(f); }, () => { if (alive) setFacets([]); });
    api.axisStats(statsQuery).then(s => { if (alive) setStats(s); }, e => { if (alive) setError(e instanceof Error ? e.message : String(e)); }).finally(() => { if (alive) setLoading(false); });
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
  const sources = useSources(api);
  const forced = forcedFacets(state.view);
  // Class đang chọn vẫn hiện (kể cả đếm 0 hoặc facet chưa tải) để người dùng bỏ chọn được.
  const classRows: ClassFacetDto[] = [
    ...facets.filter(f => !HIDDEN_CLASSES.has(f.class) && (f.count > 0 || filters.classes?.includes(f.class))),
    ...(filters.classes ?? []).filter(c => !HIDDEN_CLASSES.has(c) && !facets.some(f => f.class === c)).map(c => ({ class: c, count: 0, subclasses: [] })),
  ];
  const ranges = filters.rawAxisRanges ?? [];
  const count = ranges.length + Number(filters.variable != null) + Number(filters.italic != null) + Number(!!filters.vi) + (filters.license?.length ?? 0) + (forced.source ? 0 : filters.sourceIds?.length ?? 0) + Number(!!filters.favoritesOnly && !forced.favorites) + (filters.classes?.length ?? 0) + (filters.subclasses?.length ?? 0);
  const toggle = <K extends 'license' | 'sourceIds' | 'classes' | 'subclasses'>(key: K, v: NonNullable<LibraryFilters[K]>[number], on: boolean) => {
    const cur = (useUiStore.getState().filters[key] ?? []) as Array<typeof v>;
    const next = on ? [...cur, v] : cur.filter(x => x !== v);
    const { [key]: _, ...rest } = useUiStore.getState().filters;
    state.setFilters(next.length ? { ...rest, [key]: next } as LibraryFilters : rest);
  };
  const update = (axis: string, range: [number, number] | null) => {
    const others = (useUiStore.getState().filters.rawAxisRanges ?? []).filter(r => r.axis !== axis);
    state.setFilters({ ...useUiStore.getState().filters, rawAxisRanges: range ? [...others, { axis, range }] : others });
  };
  return <div className="fa-filter-root" ref={root}>
    <button ref={trigger} type="button" className="fa-vi-toggle" aria-expanded={open} aria-controls="font-filters" data-selected={count > 0} onClick={() => setOpen(v => !v)}>Lọc{count ? ` · ${count}` : ''}</button>
    {open && <div ref={dialog} id="font-filters" className="fa-filters" role="dialog" aria-label="Bộ lọc font">
      <header><strong>Bộ lọc</strong><Tooltip label="Đóng bộ lọc" shortcut="Esc"><button type="button" aria-label="Đóng bộ lọc" onClick={() => { setOpen(false); trigger.current?.focus(); }}>×</button></Tooltip></header>
      <div className="fa-filter-content" aria-busy={loading}>
        <fieldset><legend>{t('filter.vi')}</legend>
          {VI_OPTIONS.map(([v, k]) => <label key={v}><input type="radio" name="fa-vi" checked={filters.vi === v} onChange={() => state.setFilters({ ...filters, vi: v })} />{t(k)}</label>)}
          {filters.vi && <button type="button" className="fa-linkbtn" onClick={() => { const { vi: _, ...rest } = filters; state.setFilters(rest); }}>{t('filter.vi.clear')}</button>}
        </fieldset>
        {classRows.length > 0 && <fieldset role="group" aria-label={t('filter.class')}><legend>{t('filter.class')}</legend>
          {classRows.map(f => <div key={f.class} className="fa-class-opt">
            <label title={classGloss(f.class)}><input type="checkbox" checked={filters.classes?.includes(f.class) ?? false} onChange={e => {
              toggle('classes', f.class, e.target.checked);
              if (!e.target.checked) { const cur = useUiStore.getState().filters; const keep = (cur.subclasses ?? []).filter(x => !f.subclasses.some(sc => sc.subclass === x)); const { subclasses: _, ...rest } = cur; state.setFilters(keep.length ? { ...rest, subclasses: keep } : rest); }
            }} />{classLabel(f.class)} <span className="fa-count">{f.count}</span></label>
            {filters.classes?.includes(f.class) && f.subclasses.length > 0 && <div className="fa-sub-opts">{f.subclasses.map(sc => <label key={sc.subclass}><input type="checkbox" checked={filters.subclasses?.includes(sc.subclass) ?? false} onChange={e => toggle('subclasses', sc.subclass, e.target.checked)} />{subclassLabel(sc.subclass)} <span className="fa-count">{sc.count}</span></label>)}</div>}
          </div>)}
        </fieldset>}
        <fieldset><legend>{t('filter.license')}</legend>
          {LICENSE_OPTIONS.map(([v, k]) => <label key={v}><input type="checkbox" checked={filters.license?.includes(v) ?? false} onChange={e => toggle('license', v, e.target.checked)} />{t(k)}</label>)}
        </fieldset>
        {sources.length > 0 && <fieldset><legend>{t('filter.source')}</legend>
          {sources.map(src => <label key={src.id}><input type="checkbox" disabled={forced.source} title={forced.source ? t('filter.forced') : undefined} checked={filters.sourceIds?.includes(src.id) ?? false} onChange={e => toggle('sourceIds', src.id, e.target.checked)} />{src.name}</label>)}
        </fieldset>}
        <fieldset><legend>{t('filter.kind')}</legend>
          <label><input type="checkbox" disabled={forced.favorites} title={forced.favorites ? t('filter.forced') : undefined} checked={!!filters.favoritesOnly || forced.favorites} onChange={e => { const { favoritesOnly: _, ...rest } = filters; state.setFilters(e.target.checked ? { ...rest, favoritesOnly: true } : rest); }} />{t('filter.favorites')}</label>
          <label title={t('filter.variable.gloss')}><input type="checkbox" checked={filters.variable === true} onChange={e => state.setFilters({ ...filters, variable: e.target.checked ? true : undefined })} />{t('filter.variable')}</label>
          <label title={t('filter.italic.gloss')}><input type="checkbox" checked={filters.italic === true} onChange={e => state.setFilters({ ...filters, italic: e.target.checked ? true : undefined })} />{t('filter.italic')}</label>
        </fieldset>
        <h3>Số đo</h3>
        <p className="fa-filter-help">Chỉ font có số đo phù hợp. Các điều kiện kết hợp cùng nhau.</p>
        {error && <p role="alert">Không tải được miền số đo: {error} <button type="button" onClick={() => setRetry(n => n + 1)}>Thử lại</button></p>}
        {loading && <p role="status">{stats ? 'Đang cập nhật miền số đo…' : 'Đang đọc thư viện…'}</p>}
        {stats?.map(s => <AxisRange key={s.axis} stats={s} selected={ranges.find(r => r.axis === s.axis)?.range} onChange={range => update(s.axis, range)} />)}
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
  const min = empty ? selected?.[0] ?? 0 : roundedValue(s.min!, s.unit, 'down');
  const max = empty ? selected?.[1] ?? 0 : roundedValue(s.max!, s.unit, 'up');
  const focusMin = roundedValue(s.p1 ?? min, s.unit, 'down');
  const focusMax = roundedValue(s.p99 ?? max, s.unit, 'up');
  const range: [number, number] = selected ?? (full || constant ? [min, max] : [focusMin, focusMax]);
  const [draft, setDraft] = useState(() => range.map(v => numberText(v, s.unit)));
  const [pending, setPending] = useState<[number, number] | null>(null);
  const pendingRef = useRef<[number, number] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rangeKey = JSON.stringify(range);
  useEffect(() => { setDraft(range.map(v => numberText(v, s.unit))); setError(null); setPending(null); pendingRef.current = null; }, [rangeKey, s.unit]);
  const shown = pending ?? range;
  const outlier = !!selected && (selected[0] < focusMin || selected[1] > focusMax);
  // Giữ cận đã chọn nếu view mới có domain nhỏ hơn, không để native range tự clamp.
  const lo = Math.min(full || outlier ? min : focusMin, shown[0]);
  const hi = Math.max(full || outlier ? max : focusMax, shown[1]);
  const commit = () => {
    if (draft.every((v, i) => v === numberText(range[i], s.unit))) return;
    const [a, b] = draft.map((v, i) => v.trim() ? roundedValue(Number(v.replace(',', '.')), s.unit, i === 0 ? 'down' : 'up') : NaN);
    if (!Number.isFinite(a) || !Number.isFinite(b) || a > b || a < Math.min(min, selected?.[0] ?? min) || b > Math.max(max, selected?.[1] ?? max)) {
      setError('Nhập khoảng hợp lệ trong min–max.'); return;
    }
    setDraft([numberText(a, s.unit), numberText(b, s.unit)]);
    setError(null); onChange([a, b]);
  };
  const commitSlider = () => {
    const next = pendingRef.current;
    if (!next) return;
    pendingRef.current = null; setPending(null); onChange(next);
  };
  return <fieldset className="fa-axis-filter">
    <legend><label><input type="checkbox" checked={!!selected} disabled={empty && !selected} onChange={e => onChange(e.target.checked ? [range[0], range[1]] : null)} />{label}</label></legend>
    <div className="fa-axis-domain"><span>{unitText(s.unit)} · {s.count} font</span>{!empty && !constant && <Tooltip label={outlier ? 'Khoảng đang chọn có ngoại lệ ngoài P1–P99' : full ? 'Quay lại khoảng phổ biến P1–P99, bỏ qua các giá trị ngoại lệ' : 'Hiện cả giá trị nhỏ nhất đến lớn nhất, kể cả ngoại lệ hiếm'}><button type="button" aria-pressed={full || outlier} aria-disabled={outlier || undefined} onClick={() => { if (outlier) return; if (!full) { setFull(true); if (selected) onChange([min, max]); } else setFull(false); }}>{outlier ? 'Đang dùng min–max' : full ? 'Khoảng P1–P99' : 'Toàn khoảng'}</button></Tooltip>}</div>
    {empty && <p>Chưa có số đo trong phạm vi này</p>}
    {(!empty || selected) && <>
      <div className="fa-range-inputs">{[0, 1].map(i => <label key={i}><span>{i === 0 ? 'Từ' : 'Đến'}</span><input type="text" inputMode="decimal" aria-label={`${label} ${i === 0 ? 'từ' : 'đến'}`} disabled={!selected} value={draft[i]} onChange={e => { const next = [...draft]; next[i] = e.target.value; setDraft(next); }} onBlur={commit} onKeyDown={e => { if (e.key === 'Enter') commit(); }} /></label>)}</div>
      <div className="fa-range-sliders">{[0, 1].map(i => <input key={i} type="range" aria-label={`${label} ${i === 0 ? 'cận dưới' : 'cận trên'}`} aria-valuetext={`${numberText(shown[i], s.unit)} ${unitText(s.unit)}`} disabled={!selected || empty || constant} min={lo} max={hi} step="any" value={shown[i]} onChange={e => {
        const n = roundedValue(Number(e.target.value), s.unit, i === 0 ? 'down' : 'up');
        const next: [number, number] = i === 0 ? [Math.min(n, shown[1]), shown[1]] : [shown[0], Math.max(n, shown[0])];
        pendingRef.current = next; setPending(next); setDraft(next.map(v => numberText(v, s.unit)));
      }} onPointerUp={commitSlider} onPointerCancel={() => { pendingRef.current = null; setPending(null); setDraft(range.map(v => numberText(v, s.unit))); }} onKeyUp={commitSlider} onBlur={commitSlider} />)}</div>
      {!empty && <p>{constant ? `Cùng giá trị ${numberText(s.min!, s.unit)}; thanh trượt không cần chỉnh.` : `Min ${numberText(min, s.unit)} · max ${numberText(max, s.unit)}`}</p>}
      {error && <p role="alert">{error}</p>}
    </>}
  </fieldset>;
}
