import { useEffect, useRef, useState } from 'react';
import { Icon } from '../sidebar/icons';
import { useUiStore } from '../../store/ui';
import { Tooltip } from '../ui/Tooltip';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { FilterSummary } from '../filter/FilterSummary';
import { ActiveFiltersPopover } from '../filter/ActiveFiltersPopover';
import { groupFilters, type Group, type GroupId } from '../filter/groupFilters';
import { useFilterHistory } from '../filter/useFilterHistory';
import { useSourcesStore } from '../../store/sources';

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
  const { undo } = useFilterHistory();
  const [popOpen, setPopOpen] = useState(false);
  const [selGroup, setSelGroup] = useState<GroupId | null>(null);
  const toast = (text: string) => useNoticeStore.getState().push({ kind: 'info', text, action: { label: t('fhist.undo'), run: () => { undo(); } } });
  const dropGroup = (g: Group) => { setFilters(g.remove(filters)); setSelGroup(null); toast(t('fhist.toast', { label: g.label })); };
  const clearAll = () => { setFilters({}); setSelGroup(null); setPopOpen(false); toast(t('fhist.toastAll')); };
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
    <label className="fa-search" data-has={text !== ''}>
      <Icon name="search" size={16} />
      <input ref={input} value={text} placeholder="Tìm font" autoComplete="off" aria-label="Tìm font"
        onChange={e => onChange(e.target.value)} onKeyDown={e => {
          if (e.key === 'Escape' && text !== '') { e.preventDefault(); commit(''); return; }
          if (e.key === 'Backspace' && text === '' && groups.length) {
            e.preventDefault();
            if (e.metaKey || e.ctrlKey) { clearAll(); return; }
            const last = groups[groups.length - 1];
            if (selGroup === last.id) dropGroup(last); else setSelGroup(last.id);
            return;
          }
          if (selGroup && !['Shift', 'Meta', 'Control', 'Alt'].includes(e.key)) setSelGroup(null);
        }} />
      <FilterSummary groups={groups} selected={selGroup} open={popOpen} onOpen={() => setPopOpen(o => !o)} />
      {popOpen && groups.length > 0 && <ActiveFiltersPopover groups={groups} onClose={() => setPopOpen(false)} onClearAll={clearAll} onRemoveGroup={dropGroup}
        onRemoveChip={(g, id) => { const c = g.chips.find(x => x.id === id); if (c) { setFilters(c.remove(filters)); toast(t('fhist.toast', { label: c.label })); } }} />}
      {text !== ''
        ? <Tooltip label="Xoá tìm kiếm" shortcut="Esc"><button type="button" className="fa-clear" aria-label="Xoá tìm kiếm" onClick={() => { commit(''); input.current?.focus(); }}><Icon name="x" size={13} /></button></Tooltip>
        : <span className="fa-kbd">⌘K</span>}
    </label>
  );
}
