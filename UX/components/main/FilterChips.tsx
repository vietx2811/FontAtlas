import { useEffect } from 'react';
import type { Api, SourceDto } from '../../api';
import type { LibraryFilters } from '../../store/ui';
import { useSourcesStore } from '../../store/sources';
import { useScanStore } from '../../store/scan';
import { t } from '../../i18n';
import { classLabel, subclassLabel } from './classLabels';
import { LICENSE_OPTIONS, VI_OPTIONS, forcedFacets } from './filterOptions';
import { langName } from '../filter/langNames';
import { weightName, widthName } from '../filter/classNames';
import { AXIS_META, numberText, unitText } from '../detail/tabs/measurements';

export type Chip = { id: string; label: string; remove(f: LibraryFilters): LibraryFilters };

/** Bộ lọc đang bật → danh sách chip (thứ tự cố định); mỗi chip tự biết cách gỡ chính nó. */
export function filterChips(f: LibraryFilters, sources: SourceDto[] = [], view = 'all'): Chip[] {
  const forced = forcedFacets(view);
  const out: Chip[] = [];
  const drop = (k: keyof LibraryFilters) => (x: LibraryFilters) => { const { [k]: _, ...rest } = x; return rest; };
  if (f.vi) out.push({ id: 'vi', label: VI_OPTIONS.find(o => o[0] === f.vi) ? t(VI_OPTIONS.find(o => o[0] === f.vi)![1]) : f.vi, remove: drop('vi') });
  for (const l of f.license ?? []) out.push({ id: `license:${l}`, label: LICENSE_OPTIONS.find(o => o[0] === l) ? t(LICENSE_OPTIONS.find(o => o[0] === l)![1]) : l, remove: x => { const next = (x.license ?? []).filter(v => v !== l); const { license: _, ...rest } = x; return next.length ? { ...rest, license: next } : rest; } });
  if (f.variable != null) out.push({ id: 'variable', label: t('filter.variable'), remove: drop('variable') });
  if (f.italic != null) out.push({ id: 'italic', label: t('filter.italic'), remove: drop('italic') });
  if (f.favoritesOnly && !forced.favorites) out.push({ id: 'favorites', label: t('filter.favorites'), remove: drop('favoritesOnly') });
  for (const id of forced.source ? [] : f.sourceIds ?? []) out.push({ id: `source:${id}`, label: sources.find(s => s.id === id)?.name ?? `Nguồn ${id}`, remove: x => { const next = (x.sourceIds ?? []).filter(v => v !== id); const { sourceIds: _, ...rest } = x; return next.length ? { ...rest, sourceIds: next } : rest; } });
  for (const c of f.classes ?? []) out.push({ id: `class:${c}`, label: classLabel(c), remove: x => { const next = (x.classes ?? []).filter(v => v !== c); const { classes: _, ...rest } = x; return next.length ? { ...rest, classes: next } : rest; } });
  if (f.includeSecondary) out.push({ id: 'class:+hybrid', label: t('filter.hybrid'), remove: drop('includeSecondary') });
  if (f.confidentOnly) out.push({ id: 'class:+confident', label: t('filter.confident'), remove: drop('confidentOnly') });
  for (const c of f.subclasses ?? []) out.push({ id: `subclass:${c}`, label: subclassLabel(c), remove: x => { const next = (x.subclasses ?? []).filter(v => v !== c); const { subclasses: _, ...rest } = x; return next.length ? { ...rest, subclasses: next } : rest; } });
  if (f.weightClass) out.push({ id: 'weightClass', label: `Weight ${f.weightClass[0] === f.weightClass[1] ? weightName(f.weightClass[0]) : `${weightName(f.weightClass[0])}–${weightName(f.weightClass[1])}`}`, remove: x => { const { weightClass: _w, varAxisMatch: vm, ...rest } = x; return vm && x.widthClass ? { ...rest, varAxisMatch: vm } : rest; } });
  if (f.widthClass) out.push({ id: 'widthClass', label: `Width ${f.widthClass[0] === f.widthClass[1] ? widthName(f.widthClass[0]) : `${widthName(f.widthClass[0])}–${widthName(f.widthClass[1])}`}`, remove: x => { const { widthClass: _w, varAxisMatch: vm, ...rest } = x; return vm && x.weightClass ? { ...rest, varAxisMatch: vm } : rest; } });
  for (const l of f.langs ?? []) out.push({ id: `lang:${l.code}`, label: `${langName(l)} · ${t(l.level === 'full' ? 'lang.full' : 'lang.near')}`, remove: x => { const next = (x.langs ?? []).filter(v => v.code !== l.code); const { langs: _l, langMode: _m, ...rest } = x; return next.length ? { ...rest, langs: next, ...(x.langMode ? { langMode: x.langMode } : {}) } : rest; } });
  for (const r of f.rawAxisRanges ?? []) {
    const unit = r.axis === 'slant_deg' || r.axis === 'aperture_deg' ? 'deg' : null;
    out.push({ id: `axis:${r.axis}`, label: `${AXIS_META[r.axis]?.label ?? r.axis} ${numberText(r.range[0], unit)}–${numberText(r.range[1], unit)}${unit ? unitText(unit) : ''}`, remove: x => { const next = (x.rawAxisRanges ?? []).filter(a => a.axis !== r.axis); const { rawAxisRanges: _, ...rest } = x; return next.length ? { ...rest, rawAxisRanges: next } : rest; } });
  }
  return out;
}

export function useSources(api: Api): SourceDto[] {
  const sources = useSourcesStore(x => x.sources);
  const version = useScanStore(x => x.version);
  useEffect(() => { void useSourcesStore.getState().load(api, `${version}`); }, [api, version]);
  return sources;
}
