import { useEffect, useState } from 'react';
import type { Api, SourceDto } from '../../api';
import { useUiStore, type LibraryFilters } from '../../store/ui';
import { AXIS_META, numberText, unitText } from '../detail/tabs/measurements';
import { Icon } from '../sidebar/icons';
import './filters.css';

export const LICENSE_OPTIONS: ReadonlyArray<[string, string]> = [['ofl', 'OFL'], ['apache', 'Apache'], ['commercial', 'Thương mại'], ['unknown', 'Chưa rõ']];
export const VI_OPTIONS: ReadonlyArray<['full' | 'supported', string]> = [['full', 'Đủ dấu'], ['supported', 'Có dấu cơ bản']];

export type Chip = { id: string; label: string; remove(f: LibraryFilters): LibraryFilters };

/** Bộ lọc đang bật → danh sách chip (thứ tự cố định); mỗi chip tự biết cách gỡ chính nó. */
export function filterChips(f: LibraryFilters, sources: SourceDto[] = []): Chip[] {
  const out: Chip[] = [];
  const drop = (k: keyof LibraryFilters) => (x: LibraryFilters) => { const { [k]: _, ...rest } = x; return rest; };
  if (f.vi) out.push({ id: 'vi', label: VI_OPTIONS.find(o => o[0] === f.vi)?.[1] ?? f.vi, remove: drop('vi') });
  for (const l of f.license ?? []) out.push({ id: `license:${l}`, label: LICENSE_OPTIONS.find(o => o[0] === l)?.[1] ?? l, remove: x => { const next = (x.license ?? []).filter(v => v !== l); const { license: _, ...rest } = x; return next.length ? { ...rest, license: next } : rest; } });
  if (f.variable != null) out.push({ id: 'variable', label: 'Variable', remove: drop('variable') });
  if (f.italic != null) out.push({ id: 'italic', label: 'In nghiêng', remove: drop('italic') });
  if (f.favoritesOnly) out.push({ id: 'favorites', label: 'Yêu thích', remove: drop('favoritesOnly') });
  for (const id of f.sourceIds ?? []) out.push({ id: `source:${id}`, label: sources.find(s => s.id === id)?.name ?? `Nguồn ${id}`, remove: x => { const next = (x.sourceIds ?? []).filter(v => v !== id); const { sourceIds: _, ...rest } = x; return next.length ? { ...rest, sourceIds: next } : rest; } });
  for (const r of f.rawAxisRanges ?? []) {
    const unit = r.axis === 'slant_deg' || r.axis === 'aperture_deg' ? 'deg' : null;
    out.push({ id: `axis:${r.axis}`, label: `${AXIS_META[r.axis]?.label ?? r.axis} ${numberText(r.range[0], unit)}–${numberText(r.range[1], unit)}${unit ? unitText(unit) : ''}`, remove: x => { const next = (x.rawAxisRanges ?? []).filter(a => a.axis !== r.axis); const { rawAxisRanges: _, ...rest } = x; return next.length ? { ...rest, rawAxisRanges: next } : rest; } });
  }
  return out;
}

export function useSources(api: Api): SourceDto[] {
  const [s, setS] = useState<SourceDto[]>([]);
  useEffect(() => { let alive = true; api.listSources().then(v => { if (alive) setS(v); }, () => {}); return () => { alive = false; }; }, [api]);
  return s;
}

export function FilterChips({ api }: { api: Api }): React.JSX.Element | null {
  const filters = useUiStore(s => s.filters);
  const setFilters = useUiStore(s => s.setFilters);
  const sources = useSources(api);
  const chips = filterChips(filters, sources);
  if (chips.length === 0) return null;
  return <ul className="fa-chips" aria-label="Bộ lọc đang bật">
    {chips.map(c => <li key={c.id} className="fa-chip"><span>{c.label}</span>
      <button type="button" aria-label={`Bỏ lọc ${c.label}`} onClick={() => setFilters(c.remove(useUiStore.getState().filters))}><Icon name="x" size={12} /></button></li>)}
  </ul>;
}
