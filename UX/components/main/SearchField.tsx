import { useEffect, useRef, useState } from 'react';
import { Icon } from '../sidebar/icons';
import { useUiStore } from '../../store/ui';
import { Tooltip } from '../ui/Tooltip';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { FilterSummary } from '../filter/FilterSummary';
import { ActiveFiltersPopover } from '../filter/ActiveFiltersPopover';
import { groupFilters, restoreChanged, type Group, type GroupId } from '../filter/groupFilters';
import type { LibraryFilters } from '../../store/ui';
import { useFilterHistory } from '../filter/useFilterHistory';
import { useSourcesStore } from '../../store/sources';
import { AdvancedFunnel } from '../filter/AdvancedFilters';

export const SEARCH_DEBOUNCE_MS = 120;

export function SearchField(): React.JSX.Element {
  const query = useUiStore(s => s.query);
  const setQuery = useUiStore(s => s.setQuery);
  const [text, setText] = useState(query);
  const input = useRef<HTMLInputElement>(null);
  const filters = useUiStore(s => s.filters);
  const setFilters = useUiStore(s => s.setFilters);
  const view = useUiStore(s => s.view);
  const sources = useSourcesStore(s => s.sources);
  const groups = groupFilters(filters, sources, view);
  useFilterHistory();
  const [popOpen, setPopOpen] = useState(false);
  const [selGroup, setSelGroup] = useState<GroupId | null>(null);
  const label = useRef<HTMLLabelElement>(null);
  // Hoàn tác của toast khôi phục đúng những khoá thao tác đó đã gỡ (trên bộ lọc hiện tại), không pop trạng thái của thao tác gần nhất.
  const apply = (before: LibraryFilters, after: LibraryFilters, text: string) => {
    setFilters(after); setSelGroup(null);
    useNoticeStore.getState().push({ kind: 'info', text, action: { label: t('fhist.undo'), run: () => { const s = useUiStore.getState(); s.setFilters(restoreChanged(before, after, s.filters)); } } });
  };
  const dropGroup = (g: Group) => apply(filters, g.remove(filters), t('fhist.toast', { label: g.label }));
  const clearAll = () => { setPopOpen(false); apply(filters, {}, t('fhist.toastAll')); };
  useEffect(() => { if (!groups.length) { setPopOpen(false); setSelGroup(null); } }, [groups.length]);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // Store đổi từ nơi khác (ví dụ xoá bộ lọc) → đồng bộ ô nhập.
  useEffect(() => { setText(query); }, [query]);

  // ⌘K / Ctrl+K focus từ bất kỳ đâu.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        input.current?.focus();
        input.current?.select();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);
  useEffect(() => () => clearTimeout(timer.current), []);

  const commit = (v: string) => { clearTimeout(timer.current); setText(v); setQuery(v); };
  const onChange = (v: string) => {
    setText(v);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setQuery(v), SEARCH_DEBOUNCE_MS);
  };

  return (
    <label ref={label} className="fa-search" data-has={text !== ''}>
      <Icon name="search" size={16} />
      <input ref={input} value={text} placeholder={t('search.placeholder')} autoComplete="off" aria-label={t('search.placeholder')}
        onChange={e => onChange(e.target.value)} onKeyDown={e => {
          if (e.key === 'Escape' && text !== '') { e.preventDefault(); commit(''); return; }
          if (e.key === 'Backspace' && text === '' && groups.length) {
            e.preventDefault();
            if (e.repeat) return; // giữ phím không được xoá dần cả các nhóm
            if (e.metaKey || e.ctrlKey) { clearAll(); return; }
            const last = groups[groups.length - 1];
            if (selGroup === last.id) dropGroup(last); else setSelGroup(last.id);
            return;
          }
          if (selGroup && !['Shift', 'Meta', 'Control', 'Alt'].includes(e.key)) setSelGroup(null);
        }} />
      <FilterSummary groups={groups} selected={selGroup} open={popOpen} onOpen={() => setPopOpen(o => !o)} />
      {popOpen && groups.length > 0 && <ActiveFiltersPopover anchor={label.current} groups={groups} onClose={() => setPopOpen(false)} onClearAll={clearAll} onRemoveGroup={dropGroup}
        onRemoveChip={(g, id) => { const c = g.chips.find(x => x.id === id); if (c) apply(filters, c.remove(filters), t('fhist.toast', { label: c.label })); }} />}
      {text !== ''
        ? <Tooltip label={t('search.clear')} shortcut="Esc"><button type="button" className="fa-clear" aria-label={t('search.clear')} onClick={() => { commit(''); input.current?.focus(); }}><Icon name="x" size={13} /></button></Tooltip>
        : <span className="fa-kbd">⌘K</span>}
      <AdvancedFunnel />
    </label>
  );
}
