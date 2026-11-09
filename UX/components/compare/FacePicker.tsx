import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Api } from '../../api';
import type { FaceRow } from '../../features/library-grid/types';
import { t } from '../../i18n';
import { Icon } from '../sidebar/icons';
import './facePicker.css';

const DEBOUNCE_MS = 150;
const LIMIT = 30;
export const rowKey = (r: { hashHex: string; faceIndex: number }) => `${r.hashHex}:${r.faceIndex}`;

/** Hộp tìm font trong thư viện (queryFaces) để thêm vào so sánh. Font đã có trong so sánh không hiện. */
export function FacePicker({ api, anchor, exclude, onPick, onClose }: {
  api: Api; anchor: DOMRect; exclude: ReadonlySet<string>; onPick(r: FaceRow): void; onClose(): void;
}): React.JSX.Element {
  const [q, setQ] = useState('');
  const [items, setItems] = useState<FaceRow[] | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);

  useLayoutEffect(() => {
    const el = box.current; if (!el) return;
    const w = el.offsetWidth; const h = el.offsetHeight;
    const left = Math.min(Math.max(8, anchor.right - w), Math.max(8, window.innerWidth - w - 8));
    const below = anchor.bottom + 6 + h <= window.innerHeight - 8;
    el.style.left = `${Math.round(left)}px`;
    el.style.top = `${Math.round(below ? anchor.bottom + 6 : Math.max(8, anchor.top - 6 - h))}px`;
  }, [anchor, items]);

  useEffect(() => { input.current?.focus(); }, []);
  useEffect(() => {
    let alive = true; setItems(null);
    const id = setTimeout(() => {
      api.queryFaces({ text: q.trim() || undefined, sort: q.trim() ? 'match' : 'family', offset: 0, limit: LIMIT }).then(p => { if (alive) setItems(p.items); }, () => { if (alive) setItems([]); });
    }, q ? DEBOUNCE_MS : 0);
    return () => { alive = false; clearTimeout(id); };
  }, [api, q]);
  useEffect(() => {
    const away = (e: PointerEvent) => { if (!box.current?.contains(e.target as Node)) onClose(); };
    document.addEventListener('pointerdown', away, true);
    return () => document.removeEventListener('pointerdown', away, true);
  }, [onClose]);

  const shown = (items ?? []).filter(r => !exclude.has(rowKey(r)));
  return createPortal(
    <div ref={box} className="cmp-picker" role="dialog" aria-label={t('compare.picker.label')} onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onClose(); } }}>
      <label className="cmp-picker-q"><Icon name="search" size={14} />
        <input ref={input} type="search" value={q} placeholder={t('compare.picker.search')} aria-label={t('compare.picker.search')} onChange={e => setQ(e.target.value)} />
      </label>
      <ul role="listbox" aria-label={t('compare.picker.label')}>
        {items === null && <li className="cmp-picker-n" role="status">{t('compare.picker.loading')}</li>}
        {items !== null && shown.length === 0 && <li className="cmp-picker-n">{t('compare.picker.empty')}</li>}
        {shown.map(r => (
          <li key={rowKey(r)} role="option" aria-selected={false}>
            <button type="button" onClick={() => onPick(r)}><b>{r.family}</b><span>{r.subfamily ?? ''}</span></button>
          </li>
        ))}
      </ul>
    </div>,
    document.body,
  );
}
