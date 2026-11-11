import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { Api } from '../../api';
import type { PickMods } from '../../features/library-grid';
import type { FaceRow, GridQuery } from '../../features/library-grid/types';
import { useNoticeStore } from '../../store/notices';
import { SELECT_MAX, clearSelection, rowKey, useSelectionStore } from '../../store/selection';
import { useUiStore } from '../../store/ui';
import { t, formatNumber } from '../../i18n';

const CHUNK = 500;
/** Tìm lại mốc Shift-click: quét tối đa chừng này hàng. */
const LOCATE_MAX = 20_000;
const fmt = (n: number) => formatNumber(n);
const say = (kind: 'info' | 'error', text: string) => void useNoticeStore.getState().push({ kind, text });

/**
 * Đa chọn trên lưới: click = một, ⌘/Ctrl = bật tắt, Shift = dải theo index (lấy hàng còn thiếu từ backend),
 * ⌘A = cả truy vấn (≤ SELECT_MAX), Esc = bỏ chọn. Vùng chọn xoá khi `scopeKey` (view/truy vấn) đổi.
 */
export function useLibrarySelection(api: Api, gridQuery: GridQuery, scopeKey: string, container: React.RefObject<HTMLElement | null>, hidden?: React.RefObject<ReadonlyArray<[number, number]>>) {
  const rows = useSelectionStore(s => s.rows);
  const keys = useMemo(() => new Set(rows.keys()) as ReadonlySet<string>, [rows]);
  const queryRef = useRef(gridQuery); queryRef.current = gridQuery;
  const token = useRef(0);
  /** Face nằm trong nhóm Family đang thu gọn: không thấy trên lưới nên dải / chọn tất cả bỏ qua. */
  const isHidden = (i: number) => !!hidden?.current?.some(([a, b]) => i >= a && i <= b);

  useEffect(() => { token.current++; useSelectionStore.getState().clear(); }, [scopeKey]);
  // Truy vấn đổi (kể cả thành viên Gần đây/Đang bật): mốc cũ không còn nghĩa.
  const queryKey = JSON.stringify(gridQuery);
  useEffect(() => { useSelectionStore.getState().setAnchor(null); }, [queryKey]);
  // Mở font từ nơi khác (tab Tương tự…) thì vùng chọn cũ không còn là "vùng chọn của font đang xem".
  const selectedKey = useUiStore(s => s.selectedId);
  useEffect(() => {
    const st = useSelectionStore.getState();
    if (selectedKey && st.rows.size && !st.rows.has(selectedKey)) st.clear();
  }, [selectedKey]);

  const fetchRange = useCallback(async (lo: number, hi: number): Promise<FaceRow[]> => {
    const out: FaceRow[] = [];
    for (let off = lo; off <= hi; off += CHUNK) {
      const page = await api.queryFaces({ ...queryRef.current, offset: off, limit: Math.min(CHUNK, hi - off + 1) });
      out.push(...page.items);
      if (!page.items.length) break;
    }
    return out;
  }, [api]);

  /** Index hiện tại của một khoá trong truy vấn hiện tại (quét theo trang, có trần); null nếu không còn. */
  const locate = useCallback(async (key: string): Promise<number | null> => {
    for (let off = 0; off < LOCATE_MAX; off += CHUNK) {
      const page = await api.queryFaces({ ...queryRef.current, offset: off, limit: CHUNK });
      const i = page.items.findIndex(r => rowKey(r) === key);
      if (i >= 0) return off + i;
      if (page.items.length < CHUNK) break;
    }
    return null;
  }, [api]);

  const clear = clearSelection;

  const pick = useCallback((row: FaceRow, index: number, mods: PickMods) => {
    const st = useSelectionStore.getState();
    const ui = useUiStore.getState();
    const key = rowKey(row);
    const mine = ++token.current;
    const epoch = st.epoch;
    const stale = () => token.current !== mine || useSelectionStore.getState().epoch !== epoch;
    const plain = () => { useSelectionStore.getState().replace([row], { key, index }); useUiStore.getState().select(key); };
    if (mods.shift && st.anchor) {
      const anchor = st.anchor;
      (async () => {
        // Thứ tự có thể đã đổi từ lúc đặt mốc: kiểm index đã nhớ bằng khoá, lệch thì tìm lại; mất hẳn thì coi như click thường.
        const [at] = await fetchRange(anchor.index, anchor.index);
        const aIdx = at && rowKey(at) === anchor.key ? anchor.index : await locate(anchor.key);
        if (stale()) return;
        if (aIdx === null) { plain(); return; }
        let lo = Math.min(aIdx, index); let hi = Math.max(aIdx, index);
        if (hi - lo + 1 > SELECT_MAX) {
          // Dải quá lớn: giữ phần sát thẻ vừa bấm.
          if (index >= aIdx) lo = hi - SELECT_MAX + 1; else hi = lo + SELECT_MAX - 1;
          say('info', t('sel.maxPer', { n: fmt(SELECT_MAX) }));
        }
        const list = await fetchRange(lo, hi);
        if (stale()) return;
        const visible = list.filter((_, k) => !isHidden(lo + k));
        list.length = 0; list.push(...visible);
        if (!list.some(r => rowKey(r) === key)) list.push(row); // thẻ vừa bấm luôn nằm trong vùng chọn
        const next = { key: anchor.key, index: aIdx };
        if (mods.meta) st.add(list, next); else st.replace(list, next);
        useUiStore.getState().select(key);
      })().catch(e => { if (!stale()) say('error', t('sel.rangeFail', { msg: (e as Error)?.message ?? String(e) })); });
      return;
    }
    if (mods.meta) {
      const added = st.toggle(row, index);
      if (added) ui.select(key);
      else if (ui.selectedId === key) ui.select([...useSelectionStore.getState().rows.keys()].at(-1) ?? null);
      return;
    }
    plain();
  }, [fetchRange, locate]);

  const selectAll = useCallback(async () => {
    const mine = ++token.current;
    const epoch = useSelectionStore.getState().epoch;
    try {
      const first = await api.queryFaces({ ...queryRef.current, offset: 0, limit: Math.min(CHUNK, SELECT_MAX) });
      const want = Math.min(first.total, SELECT_MAX);
      const list = [...first.items];
      if (want > list.length) list.push(...await fetchRange(list.length, want - 1));
      if (token.current !== mine || useSelectionStore.getState().epoch !== epoch) return;
      const picked = list.slice(0, want).filter((_, k) => !isHidden(k));
      useSelectionStore.getState().replace(picked, picked.length ? { key: rowKey(picked[0]), index: 0 } : null);
      if (first.total > SELECT_MAX) say('info', t('sel.firstN', { n: fmt(SELECT_MAX), total: fmt(first.total) }));
    } catch (e) { if (token.current === mine) say('error', t('sel.allFail', { msg: (e as Error)?.message ?? String(e) })); }
  }, [api, fetchRange]);

  // ⌘A / Esc: chỉ khi không đang gõ chữ, không có menu/hộp thoại, và focus ở thư viện hoặc ngoài mọi vùng.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      if (useUiStore.getState().compare || useUiStore.getState().pairThese) return; // thư viện đang ẩn sau màn Compare/Pair these: phím thuộc về màn đó
      const t = e.target as HTMLElement | null;
      if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
      if (document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"]')) return;
      const inLib = !!container.current?.contains(document.activeElement) || document.activeElement === document.body || document.activeElement === null;
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'a') {
        if (!inLib) return;
        e.preventDefault();
        void selectAll();
      } else if (e.key === 'Escape' && useSelectionStore.getState().rows.size > 0) {
        clear();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [container, selectAll, clear]);

  return { rows, keys, pick, selectAll, clear };
}
