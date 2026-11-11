import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, SourceDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';

/** Id theo thứ tự ưu tiên + nguồn chưa có trong danh sách (mới thêm) xếp cuối; id không còn nguồn thì bỏ. */
export function mergeOrder(order: number[], sources: SourceDto[]): number[] {
  const ids = new Set(sources.map(s => s.id));
  const kept = order.filter(i => ids.has(i));
  return [...kept, ...sources.map(s => s.id).filter(i => !kept.includes(i))];
}
export const moveItem = <T,>(list: T[], from: number, to: number): T[] => {
  if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return list;
  const next = list.slice(); const [x] = next.splice(from, 1); next.splice(to, 0, x); return next;
};

/**
 * Thứ tự nguồn quyết định bản được giữ khi hai nguồn có cùng một font. Kéo tay cầm để đổi (con trỏ), hoặc dùng phím ↑/↓ trên tay cầm
 * hay hai nút "lên/xuống". Mỗi lần đổi gửi TOÀN BỘ danh sách; lỗi → trả về thứ tự cũ và báo rõ.
 */
export function SourcePriority({ api, sources }: { api: Api; sources: SourceDto[] }): React.JSX.Element | null {
  const [order, setOrder] = useState<number[]>([]);
  const [drag, setDrag] = useState<{ id: number; over: number } | null>(null);
  const [live, setLive] = useState('');
  const list = useRef<HTMLOListElement>(null);
  const seq = useRef(0);
  const rev = useUiStore(s => s.libraryRev);
  useEffect(() => { const mine = ++seq.current; api.sourcePriority().then(o => { if (mine === seq.current) setOrder(o); }, () => undefined); }, [api, rev]);
  const ids = mergeOrder(order, sources);
  const name = (id: number) => sources.find(s => s.id === id)?.name ?? String(id);

  const commit = useCallback(async (next: number[], movedId: number) => {
    const before = ids; const mine = ++seq.current;
    setOrder(next); setLive(t('dup.prio.moved', { name: name(movedId), n: next.indexOf(movedId) + 1, total: next.length }));
    try { const saved = await api.setSourcePriority(next); if (mine === seq.current) setOrder(saved); useUiStore.getState().bumpLibrary(); }
    catch (e) { if (mine === seq.current) setOrder(before); useNoticeStore.getState().push({ kind: 'error', text: t('dup.prio.fail', { msg: e instanceof Error ? e.message : String(e) }) }); }
  }, [api, ids]); // eslint-disable-line react-hooks/exhaustive-deps

  const move = (id: number, to: number) => { const from = ids.indexOf(id); if (to < 0 || to >= ids.length || to === from) return; void commit(moveItem(ids, from, to), id); };

  const onDown = (id: number) => (e: React.PointerEvent<HTMLButtonElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture?.(e.pointerId);
    setDrag({ id, over: ids.indexOf(id) });
  };
  const overIndex = (y: number): number => {
    const rows = [...(list.current?.querySelectorAll('li') ?? [])];
    for (let i = 0; i < rows.length; i++) { const r = rows[i].getBoundingClientRect(); if (y < r.top + r.height / 2) return i; }
    return Math.max(0, rows.length - 1);
  };
  const onMove = (e: React.PointerEvent) => { if (drag) setDrag({ ...drag, over: overIndex(e.clientY) }); };
  const onUp = () => { if (!drag) return; const d = drag; setDrag(null); move(d.id, d.over); };
  const onKey = (id: number) => (e: React.KeyboardEvent) => {
    const d = e.key === 'ArrowUp' ? -1 : e.key === 'ArrowDown' ? 1 : 0; if (!d) return;
    e.preventDefault(); move(id, ids.indexOf(id) + d);
  };

  if (sources.length < 2) return null;
  const shown = drag ? moveItem(ids, ids.indexOf(drag.id), drag.over) : ids;
  return (
    <section className="fa-set-prio" aria-labelledby="prio-h" data-testid="source-priority">
      <h3 id="prio-h">{t('dup.prio.title')}</h3>
      <p className="fa-set-note">{t('dup.prio.hint')}</p>
      <ol ref={list} aria-label={t('dup.prio.list')} onPointerMove={onMove} onPointerUp={onUp} onPointerCancel={() => setDrag(null)}>
        {shown.map((id, i) => (
          <li key={id} data-testid="prio-row" data-dragging={drag?.id === id}>
            <span className="pos">{t('dup.prio.pos', { n: i + 1 })}</span>
            <button type="button" className="grip" aria-label={t('dup.prio.drag', { name: name(id) })} title={t('dup.prio.drag', { name: name(id) })} onPointerDown={onDown(id)} onKeyDown={onKey(id)}>⋮⋮</button>
            <strong>{name(id)}</strong>
            <span className="mv">
              <button type="button" aria-label={t('dup.prio.up', { name: name(id) })} aria-disabled={i === 0} onClick={() => move(id, i - 1)}>↑</button>
              <button type="button" aria-label={t('dup.prio.down', { name: name(id) })} aria-disabled={i === shown.length - 1} onClick={() => move(id, i + 1)}>↓</button>
            </span>
          </li>
        ))}
      </ol>
      <p className="fa-set-note">{t('dup.prio.recheck')}</p>
      <div className="fa-sr-only" role="status" aria-live="polite">{live}</div>
    </section>
  );
}
