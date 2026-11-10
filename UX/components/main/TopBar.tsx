import { useMemo } from 'react';
import type { Api } from '../../api';
import { useUiStore } from '../../store/ui';
import { useSelectionStore } from '../../store/selection';
import { SelectionBar } from './SelectionBar';
import { Icon } from '../sidebar/icons';
import { SearchField } from './SearchField';
import { useViewSummary } from './useViewSummary';
import './topbar.css';
import { AdvancedFilters } from '../filter/AdvancedFilters';
import { filterChips, useSources } from './FilterChips';
import { TopBarActions } from './TopBarActions';
import { Tooltip } from '../ui/Tooltip';

export function TopBar({ api }: { api: Api }): React.JSX.Element {
  const { title, count } = useViewSummary(api);
  const familyScope = useUiStore(s => s.familyScope);
  const filters = useUiStore(s => s.filters);
  const setView = useUiStore(s => s.setView);
  const viOnly = useUiStore(s => s.viOnly);
  const setViOnly = useUiStore(s => s.setViOnly);
  const layout = useUiStore(s => s.layout);
  const setLayout = useUiStore(s => s.setLayout);
  const selected = useSelectionStore(s => s.rows);
  const selectedRows = useMemo(() => [...selected.values()], [selected]);
  const showSelection = selectedRows.length >= 2;
  const view = useUiStore(s => s.view);
  useSources(api); // nguồn dùng cho nhãn "Thư viện" của tóm tắt bộ lọc
  const hasFilters = filterChips(filters, [], view).length > 0;
  return (
    <header className={`fa-topbar${familyScope ? ' has-family' : ''}${familyScope || showSelection ? ' has-row' : ''}`}>
      <div className="fa-ttl"><h1>{familyScope ? 'Thư viện' : title}</h1>{count !== null && <span className="fa-cnt" data-testid="view-count">{count}</span>}</div>
      <SearchField />
      <Tooltip label="Chỉ font đủ dấu tiếng Việt"><button type="button" className="fa-vi-toggle" aria-pressed={viOnly} onClick={() => setViOnly(!viOnly)}>Ặ vi</button></Tooltip>
      <AdvancedFilters api={api} />
      <TopBarActions api={api} />
      <div className="fa-seg" role="group" aria-label="Kiểu hiển thị">
        <Tooltip label="Hiện dạng lưới"><button type="button" className="fa-iconbtn" aria-pressed={layout === 'grid'} aria-label="Lưới" onClick={() => setLayout('grid')}><Icon name="grid" size={16} /></button></Tooltip>
        <Tooltip label="Hiện dạng danh sách"><button type="button" className="fa-iconbtn" aria-pressed={layout === 'list'} aria-label="Danh sách" onClick={() => setLayout('list')}><Icon name="list" size={16} /></button></Tooltip>
      </div>
      {(familyScope || showSelection) && <div className="fa-family-row">
        {familyScope && <Tooltip label={`${count ?? '…'}/${familyScope.faces.length} kiểu${hasFilters ? ' · có lọc' : ''} — bấm để bỏ phạm vi family`}><button type="button" className="fa-family-chip" aria-label={`Bỏ phạm vi family ${familyScope.name}`} onClick={() => setView('all')}><span>Family: {familyScope.name}{hasFilters ? ' · có lọc' : ''}</span><span aria-hidden>×</span></button></Tooltip>}
        {showSelection && <SelectionBar rows={selectedRows} />}
      </div>}
    </header>
  );
}
