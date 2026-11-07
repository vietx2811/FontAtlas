import { useEffect, useRef, useState } from 'react';
import { Icon } from '../sidebar/icons';
import { useUiStore } from '../../store/ui';
import { Tooltip } from '../ui/Tooltip';

export const SEARCH_DEBOUNCE_MS = 120;

export function SearchField(): React.JSX.Element {
  const query = useUiStore(s => s.query);
  const setQuery = useUiStore(s => s.setQuery);
  const [text, setText] = useState(query);
  const input = useRef<HTMLInputElement>(null);
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
        onChange={e => onChange(e.target.value)} onKeyDown={e => { if (e.key === 'Escape' && text !== '') { e.preventDefault(); commit(''); } }} />
      {text !== ''
        ? <Tooltip label="Xoá tìm kiếm" shortcut="Esc"><button type="button" className="fa-clear" aria-label="Xoá tìm kiếm" onClick={() => { commit(''); input.current?.focus(); }}><Icon name="x" size={13} /></button></Tooltip>
        : <span className="fa-kbd">⌘K</span>}
    </label>
  );
}
