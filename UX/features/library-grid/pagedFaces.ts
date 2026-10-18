// Phân trang + lọc ở backend; bỏ phản hồi cũ; an toàn với StrictMode (tạo/huỷ trong cùng effect).
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type { LibraryApi } from './api';
import type { FaceRow, GridQuery } from './types';

export const PAGE_SIZE = 200;
export const SEARCH_DEBOUNCE_MS = 150;

export type PagedSnapshot = { total: number | null; version: number; error: string | null };

const restKey = (q: GridQuery) => { const { text: _, ...rest } = q; return JSON.stringify(rest); };

export class PagedFaces {
  private epoch = 0;
  private disposed = false;
  private query: GridQuery;
  private pages = new Map<number, FaceRow[]>();
  private inflight = new Set<number>();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private listeners = new Set<() => void>();
  private snap: PagedSnapshot = { total: null, version: 0, error: null };

  constructor(private api: Pick<LibraryApi, 'queryFaces'>, query: GridQuery) {
    this.query = query;
    this.fetchPage(0);
  }

  subscribe = (cb: () => void) => { this.listeners.add(cb); return () => { this.listeners.delete(cb); }; };
  getSnapshot = () => this.snap;

  setQuery(q: GridQuery) {
    if (this.disposed) return;
    if (this.timer) { clearTimeout(this.timer); this.timer = null; }
    if (JSON.stringify(q) === JSON.stringify(this.query)) return;
    if (restKey(q) === restKey(this.query)) {
      // chỉ đổi chữ tìm → debounce
      this.timer = setTimeout(() => { this.timer = null; this.apply(q); }, SEARCH_DEBOUNCE_MS);
    } else {
      this.apply(q); // bộ lọc khác → áp dụng ngay (timer chờ đã huỷ ở trên)
    }
  }

  private apply(q: GridQuery) {
    this.query = q;
    this.epoch++;
    this.pages.clear();
    this.inflight.clear();
    this.emit({ total: null, error: null });
    this.fetchPage(0);
  }

  ensureRange(start: number, stop: number) {
    if (this.disposed) return;
    const first = Math.floor(Math.max(0, start - PAGE_SIZE / 2) / PAGE_SIZE);
    let last = Math.floor((stop + PAGE_SIZE / 2) / PAGE_SIZE);
    const total = this.snap.total;
    if (total !== null) last = Math.min(last, Math.max(0, Math.floor((total - 1) / PAGE_SIZE)));
    if (total === 0) return;
    for (let n = first; n <= last; n++) {
      if (!this.pages.has(n) && !this.inflight.has(n)) this.fetchPage(n);
    }
  }

  rowAt(i: number): FaceRow | undefined {
    return this.pages.get(Math.floor(i / PAGE_SIZE))?.[i % PAGE_SIZE];
  }

  dispose() {
    this.disposed = true;
    if (this.timer) clearTimeout(this.timer);
    this.listeners.clear();
  }

  private fetchPage(n: number) {
    const e = this.epoch;
    this.inflight.add(n);
    this.api.queryFaces({ ...this.query, offset: n * PAGE_SIZE, limit: PAGE_SIZE }).then(
      page => {
        if (this.disposed || e !== this.epoch) return;
        this.inflight.delete(n);
        this.pages.set(n, page.items);
        this.emit({ total: page.total, error: null });
      },
      err => {
        if (this.disposed || e !== this.epoch) return;
        this.inflight.delete(n);
        this.emit({ error: err?.message ?? String(err) });
      },
    );
  }

  private emit(patch: Partial<PagedSnapshot>) {
    this.snap = { ...this.snap, ...patch, version: this.snap.version + 1 };
    this.listeners.forEach(cb => cb());
  }
}

const EMPTY: PagedSnapshot = { total: null, version: 0, error: null };
const noopSubscribe = () => () => {};

export function usePagedFaces(api: LibraryApi, query: GridQuery): { paged: PagedFaces | null; snap: PagedSnapshot } {
  const [paged, setPaged] = useState<PagedFaces | null>(null);
  const queryRef = useRef(query);
  queryRef.current = query;
  useEffect(() => {
    const p = new PagedFaces(api, queryRef.current);
    setPaged(p);
    return () => p.dispose();
  }, [api]);
  const key = JSON.stringify(query);
  useEffect(() => { paged?.setQuery(queryRef.current); }, [paged, key]);
  const snap = useSyncExternalStore(paged ? paged.subscribe : noopSubscribe, paged ? paged.getSnapshot : () => EMPTY);
  return { paged, snap };
}
