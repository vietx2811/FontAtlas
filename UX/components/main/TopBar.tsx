import type { Api } from '../../api';
import { useUiStore } from '../../store/ui';
import { Icon } from '../sidebar/icons';
import { SearchField } from './SearchField';
import { useViewSummary } from './useViewSummary';
import './topbar.css';

export function TopBar({ api }: { api: Api }): React.JSX.Element {
  const { title, count } = useViewSummary(api);
  const viOnly = useUiStore(s => s.viOnly);
  const setViOnly = useUiStore(s => s.setViOnly);
  const layout = useUiStore(s => s.layout);
  const setLayout = useUiStore(s => s.setLayout);
  return (
    <header className="fa-topbar">
      <div className="fa-ttl"><h1>{title}</h1>{count !== null && <span className="fa-cnt" data-testid="view-count">{count}</span>}</div>
      <SearchField />
      <button type="button" className="fa-vi-toggle" aria-pressed={viOnly} title="Chỉ font đủ dấu tiếng Việt" onClick={() => setViOnly(!viOnly)}>Ặ vi</button>
      <div className="fa-seg" role="group" aria-label="Kiểu hiển thị">
        <button type="button" className="fa-iconbtn" aria-pressed={layout === 'grid'} aria-label="Lưới" title="Lưới" onClick={() => setLayout('grid')}><Icon name="grid" size={16} /></button>
        <button type="button" className="fa-iconbtn" aria-pressed={layout === 'list'} aria-label="Danh sách" title="Danh sách" onClick={() => setLayout('list')}><Icon name="list" size={16} /></button>
      </div>
    </header>
  );
}
