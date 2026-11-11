import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useShallow } from 'zustand/react/shallow';
import type { Api, AxisHistogramDto, ClassFacetDto } from '../../api';
import { t } from '../../i18n';
import { toMetadataQuery, useUiStore, type LibraryFilters } from '../../store/ui';
import { useScanStore } from '../../store/scan';
import { useSourcesStore } from '../../store/sources';
import { TermTip, classTermKey, subclassTermKey } from '../ui/terms';
import { HIDDEN_CLASSES, classLabel, subclassLabel } from '../main/classLabels';
import { LICENSE_OPTIONS, VI_OPTIONS, forcedFacets } from '../main/filterOptions';
import { filterChips } from '../main/FilterChips';
import { roundedValue } from '../detail/tabs/measurements';
import { Masonry } from './Masonry';
import { AxisRow } from './AxisRow';
import { LangPicker } from './LangPicker';
import { Histogram } from './Histogram';
import { AXIS_GROUPS } from './axisGroups';
import { WEIGHT_TICKS, WIDTH_TICKS, weightName, widthName } from './classNames';
import { HISTOGRAM_DEBOUNCE_MS, useAxisHistograms } from './useAxisHistograms';
import './advancedSheet.css';

const stepFor = (unit: string): number => unit === 'deg' ? 0.1 : 0.001;

/** Sheet phủ lưới: thẻ Basic (phân loại, style, thư viện, giấy phép, tiếng Việt) + 18 trục chia nhóm. Esc đóng; bố cục masonry `columns: 208px`. */
export function AdvancedSheet({ api, onClose }: { api: Api; onClose(): void }): React.JSX.Element {
  const filters = useUiStore(s => s.filters);
  const setFilters = useUiStore(s => s.setFilters);
  const view = useUiStore(s => s.view);
  const sources = useSourcesStore(s => s.sources);
  const version = useScanStore(s => s.version);
  const ui = useUiStore(useShallow(s => ({ view: s.view, query: s.query, viOnly: s.viOnly, filters: s.filters })));
  const query = useMemo(() => toMetadataQuery(ui), [ui.view, ui.query, ui.viOnly, ui.filters]); // eslint-disable-line react-hooks/exhaustive-deps
  const { data: hist, extent, loading, error, retry } = useAxisHistograms(api, query, true, version);
  const [facets, setFacets] = useState<ClassFacetDto[]>([]);
  const root = useRef<HTMLDivElement>(null);
  const forced = forcedFacets(view);
  const opener = useRef<Element | null>(document.activeElement);

  useEffect(() => {
    root.current?.focus();
    // Esc đóng MỘT lớp: nếu popover "Đang lọc" đang mở thì để popover tự đóng trước, sheet giữ nguyên.
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape' && !document.querySelector('[data-testid=active-filters]')) { e.preventDefault(); e.stopPropagation(); onClose(); } };
    document.addEventListener('keydown', key, true);
    const from = opener.current;
    return () => { document.removeEventListener('keydown', key, true); if (from instanceof HTMLElement) from.focus(); };
  }, [onClose]);
  useEffect(() => {
    let alive = true;
    const { classes: _c, subclasses: _s, ...rest } = query;
    // Cùng nhịp debounce với histogram: kéo slider liên tục không bắn classFacets mỗi lần.
    const timer = setTimeout(() => { api.classFacets(rest).then(f => { if (alive) setFacets(f); }, () => { if (alive) setFacets([]); }); }, HISTOGRAM_DEBOUNCE_MS);
    return () => { alive = false; clearTimeout(timer); };
  }, [api, query, version]);

  const patch = (next: Partial<LibraryFilters>) => {
    const cur = useUiStore.getState().filters;
    const merged = { ...cur, ...next } as Record<string, unknown>;
    for (const k of Object.keys(merged)) if (merged[k] === undefined || (Array.isArray(merged[k]) && (merged[k] as unknown[]).length === 0)) delete merged[k];
    setFilters(merged as LibraryFilters);
  };
  const toggle = (key: 'license' | 'classes' | 'sourceIds', v: string | number, on: boolean) => {
    const cur = (useUiStore.getState().filters[key] ?? []) as Array<string | number>;
    patch({ [key]: on ? [...cur, v] : cur.filter(x => x !== v) } as Partial<LibraryFilters>);
  };
  const setRange = (axis: string, range: [number, number] | null) => {
    const others = (useUiStore.getState().filters.rawAxisRanges ?? []).filter(r => r.axis !== axis);
    patch({ rawAxisRanges: range ? [...others, { axis, range }] : others });
  };
  const setVarMatch = (k: 'wght' | 'wdth', on: boolean) => {
    const cur = { wght: true, wdth: true, ...(useUiStore.getState().filters.varAxisMatch ?? {}) };
    const next = { ...cur, [k]: on };
    patch({ varAxisMatch: next.wght && next.wdth ? undefined : next });
  };
  const varMatch = { wght: true, wdth: true, ...(filters.varAxisMatch ?? {}) };
  // Class đang chọn vẫn hiện (kể cả đếm 0 hoặc facet chưa tải) để bỏ chọn được.
  const classRows: ClassFacetDto[] = [
    ...facets.filter(f => !HIDDEN_CLASSES.has(f.class) && (f.count > 0 || filters.classes?.includes(f.class))),
    ...(filters.classes ?? []).filter(c => !HIDDEN_CLASSES.has(c) && !facets.some(f => f.class === c)).map(c => ({ class: c, count: 0, subclasses: [] })),
  ];
  const toggleClass = (f: ClassFacetDto, on: boolean) => {
    const cur = useUiStore.getState().filters;
    const classes = on ? [...(cur.classes ?? []), f.class] : (cur.classes ?? []).filter(c => c !== f.class);
    const subclasses = on ? cur.subclasses : (cur.subclasses ?? []).filter(x => !f.subclasses.some(sc => sc.subclass === x));
    patch({ classes, subclasses });
  };
  const toggleSub = (sub: string, on: boolean) => {
    const cur = useUiStore.getState().filters.subclasses ?? [];
    patch({ subclasses: on ? [...cur, sub] : cur.filter(x => x !== sub) });
  };
  const condCount = filterChips(filters, sources, view).length;
  const [edge, setEdge] = useState<{ left: number; top: number } | null>(null);
  useEffect(() => {
    // Sheet bắt đầu ngay dưới Top Bar (cao hơn khi có hàng family/chọn nhiều) và sát mép phải sidebar.
    const measure = () => setEdge({ left: Math.round(document.querySelector('[aria-label="Thanh bên"]')?.getBoundingClientRect().right ?? 0), top: Math.round(document.querySelector('.fa-topbar')?.getBoundingClientRect().bottom ?? 0) });
    measure(); window.addEventListener('resize', measure);
    return () => window.removeEventListener('resize', measure);
  }, []);

  const renderAxis = (axis: string) => {
    const h: AxisHistogramDto | undefined = hist.get(axis);
    if (axis === 'weight_class' || axis === 'width_class') {
      const isW = axis === 'weight_class';
      const value = (isW ? filters.weightClass : filters.widthClass) ?? null;
      const name = isW ? weightName : widthName;
      const which = isW ? 'wght' : 'wdth';
      return (
        <AxisRow key={axis} axis={axis} source={t('adv.osClass')} min={isW ? 100 : 1} max={isW ? 900 : 9} step={isW ? 100 : 1} ticks={isW ? WEIGHT_TICKS : WIDTH_TICKS}
          value={value as [number, number] | null} format={name} count={h?.count}
          onChange={r => patch(isW ? { weightClass: r ?? undefined } : { widthClass: r ?? undefined })}
          extra={<button type="button" className="fa-varbtn" data-testid={`varmatch-${which}`} aria-pressed={varMatch[which]} title={t('adv.varMatch', { axis: which })} onClick={() => setVarMatch(which, !varMatch[which])}>{which}</button>}>
          {h && <Histogram hist={h} />}
        </AxisRow>
      );
    }
    const sel = filters.rawAxisRanges?.find(r => r.axis === axis)?.range ?? null;
    const empty = !h || h.empty;
    const unit = h?.unit ?? '';
    const min = h ? h.lo : 0;
    const max = h ? h.hi : 1;
    const ext = extent.get(axis);
    const fullMin = ext?.min != null ? roundedValue(ext.min, unit, 'down') : undefined;
    const fullMax = ext?.max != null ? roundedValue(ext.max, unit, 'up') : undefined;
    return (
      <AxisRow key={axis} axis={axis} source={t('adv.measured')} min={min} max={max} fullMin={fullMin} fullMax={fullMax} step={stepFor(unit)} digits={unit === 'deg' ? 1 : 3}
        value={sel as [number, number] | null} empty={!!h && empty && !sel} count={h?.count} onChange={r => setRange(axis, r)}>
        {h && <Histogram hist={h} />}
      </AxisRow>
    );
  };

  return createPortal(
    <div ref={root} className="fa-adv" role="dialog" aria-label={t('adv.title')} tabIndex={-1} data-testid="advanced-sheet" aria-busy={loading} style={edge && edge.left ? { left: edge.left, top: edge.top || undefined } : undefined}>
      <header className="fa-adv-head">
        <strong>{t('adv.title')}</strong>
        <span className="fa-adv-count">{t('adv.count', { n: condCount })}</span>
        <button type="button" className="fa-adv-reset" disabled={!condCount} onClick={() => setFilters({})}>{t('adv.reset')}</button>
        <button type="button" className="fa-adv-close" aria-label={t('adv.close')} onClick={onClose}>×</button>
      </header>
      {error && <p role="alert" className="fa-adv-err">{t('adv.error', { msg: error })} <button type="button" onClick={retry}>{t('adv.retry')}</button></p>}
      <Masonry>
        <section className="fa-adv-card" role="group" aria-label={t('adv.class')} data-weight={60 + 31 * classRows.length}><h3>{t('adv.class')}</h3>
          {classRows.map(f => (
            <div key={f.class} className="fa-class-opt">
              <TermTip term={classTermKey(f.class)}>
                <label><input type="checkbox" checked={filters.classes?.includes(f.class) ?? false} onChange={e => toggleClass(f, e.target.checked)} />{classLabel(f.class)} <span className="fa-count">{f.count}</span></label>
              </TermTip>
              {filters.classes?.includes(f.class) && f.subclasses.length > 0 && <div className="fa-sub-opts">
                {f.subclasses.map(sc => <TermTip key={sc.subclass} term={subclassTermKey(sc.subclass)}>
                  <label><input type="checkbox" checked={filters.subclasses?.includes(sc.subclass) ?? false} onChange={e => toggleSub(sc.subclass, e.target.checked)} />{subclassLabel(sc.subclass)} <span className="fa-count">{sc.count}</span></label>
                </TermTip>)}
              </div>}
            </div>
          ))}
        </section>
        <section className="fa-adv-card" aria-label={t('adv.style')} data-weight={100}><h3>{t('adv.style')}</h3>
          <label title={t('filter.variable.gloss')}><input type="checkbox" checked={filters.variable === true} onChange={e => patch({ variable: e.target.checked ? true : undefined })} />{t('filter.variable')}</label>
          <label title={t('filter.italic.gloss')}><input type="checkbox" checked={filters.italic === true} onChange={e => patch({ italic: e.target.checked ? true : undefined })} />{t('filter.italic')}</label>
        </section>
        <section className="fa-adv-card" aria-label={t('adv.vi')} data-weight={110}><h3>{t('adv.vi')}</h3>
          {VI_OPTIONS.map(([v, k]) => <label key={v}><input type="radio" name="fa-adv-vi" checked={filters.vi === v} onChange={() => patch({ vi: v })} />{t(k)}</label>)}
          {filters.vi && <button type="button" className="fa-linkbtn" onClick={() => patch({ vi: undefined })}>{t('filter.vi.clear')}</button>}
        </section>
        <section className="fa-adv-card" aria-label={t('adv.lang')} data-weight={440}><h3>{t('adv.lang')}</h3><LangPicker api={api} query={query} refreshKey={version} /></section>
        <section className="fa-adv-card" aria-label={t('adv.library')} data-weight={60 + 31 * (1 + sources.length)}><h3>{t('adv.library')}</h3>
          <label><input type="checkbox" disabled={forced.favorites} checked={!!filters.favoritesOnly || forced.favorites} onChange={e => patch({ favoritesOnly: e.target.checked ? true : undefined })} />{t('filter.favorites')}</label>
          {sources.map(s => <label key={s.id}><input type="checkbox" disabled={forced.source} checked={filters.sourceIds?.includes(s.id) ?? false} onChange={e => toggle('sourceIds', s.id, e.target.checked)} />{s.name}</label>)}
        </section>
        <section className="fa-adv-card" aria-label={t('adv.license')} data-weight={150}><h3>{t('adv.license')}</h3>
          {LICENSE_OPTIONS.map(([v, k]) => <label key={v}><input type="checkbox" checked={filters.license?.includes(v) ?? false} onChange={e => toggle('license', v, e.target.checked)} />{t(k)}</label>)}
        </section>
        {AXIS_GROUPS.map(g => (
          <section key={g.id} className="fa-adv-card" aria-label={g.title} data-group={g.id} data-weight={56 + 133 * g.axes.length}><h3>{g.title}</h3>
            {g.axes.map(renderAxis)}
          </section>
        ))}
      </Masonry>
    </div>,
    document.body,
  );
}
