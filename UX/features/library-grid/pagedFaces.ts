// Phân trang + lọc ở backend; bỏ phản hồi cũ; an toàn với StrictMode (tạo/huỷ trong cùng effect).
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type { LibraryApi } from './api';
import type { FaceRow, GridQuery } from './types';

export const PAGE_SIZE = 200;
export const SEARCH_DEBOUNCE_MS = 150;

/** `query`/`queryKey`: truy vấn mà `total` và các trang thuộc về (đã áp dụng, không phải prop mới nhất đang chờ debounce); chỉ mục nhóm chỉ ghép khi khớp. */
export type PagedSnapshot = { total: number | null; version: number; error: string | null; query: GridQuery | null; queryKey: string };

const restKey = (q: GridQuery) => { const { text: _, ...rest } = q; return JSON.stringify(rest); };

export class PagedFaces {
  private epoch = 0;
  private disposed = false;
  private query: GridQuery;
  private pages = new Map<number, FaceRow[]>();
  private inflight = new Set<number>();
  /** Trang lỗi: không tự fetch lại (tránh vòng lặp lỗi → render → fetch); chỉ thử lại qua retry() hoặc đổi query. */
  private failed = new Set<number>();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private listeners = new Set<() => void>();
  private snap: PagedSnapshot;
  private lastRange: [number, number] = [0, 0];
  private refreshSeq = 0;
  /** Lần làm tươi trước đã nhận total=0 khi đang quét (và ta chưa tin): cần lần thứ hai xác nhận mới bỏ dữ liệu cũ. */
  private zeroPending = false;

  constructor(private api: Pick<LibraryApi, 'queryFaces'>, query: GridQuery) {
    this.query = query;
    this.snap = { total: null, version: 0, error: null, query, queryKey: JSON.stringify(query) };
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
    this.snap = { ...this.snap, query: q, queryKey: JSON.stringify(q) };
    this.epoch++;
    this.zeroPending = false; // số 0 đầu tiên của truy vấn mới vẫn phải qua bước xác nhận
    this.pages.clear();
    this.inflight.clear();
    this.failed.clear();
    this.emit({ total: null, error: null });
    this.fetchPage(0);
  }

  ensureRange(start: number, stop: number) {
    if (this.disposed) return;
    this.lastRange = [start, stop];
    const first = Math.floor(Math.max(0, start - PAGE_SIZE / 2) / PAGE_SIZE);
    let last = Math.floor((stop + PAGE_SIZE / 2) / PAGE_SIZE);
    const total = this.snap.total;
    if (total !== null) last = Math.min(last, Math.max(0, Math.floor((total - 1) / PAGE_SIZE)));
    if (total === 0) return;
    for (let n = first; n <= last; n++) {
      if (!this.pages.has(n) && !this.inflight.has(n) && !this.failed.has(n)) this.fetchPage(n);
    }
  }

  /**
   * Làm tươi tại chỗ (quét thêm font…): tải lại ngầm các trang đang giữ + trang 0, chỉ thay dữ liệu khi cả nhóm đã về; trong lúc chờ lưới giữ
   * nguyên trang/total (không rỗng, không mất cuộn). `scanning`: total=0 khi đang quét có thể là trạng thái ghi dở → giữ dữ liệu cũ tới lần
   * xác nhận thứ hai; khi không quét thì 0 là thật (vd đã gỡ nguồn) và hiện ngay. Đổi truy vấn (epoch) huỷ lần làm tươi đang chờ.
   */
  refresh(scanning = false) {
    if (this.disposed) return;
    const mySeq = ++this.refreshSeq;
    this.inflight.clear(); // fetch bắt đầu trước lần làm tươi sẽ bị bỏ (fetchPage kiểm refreshSeq); trang thiếu được tải lại qua ensureRange
    const e = this.epoch;
    // Chỉ tải lại các trang đang nhìn (viewport ± 1 trang) + trang 0 (nơi lấy total); trang khác bị bỏ (stale) và tải lại khi cuộn tới.
    const [a, b] = this.lastRange;
    const lo = Math.max(0, Math.floor(a / PAGE_SIZE) - 1), hi = Math.floor(b / PAGE_SIZE) + 1;
    const want = [...new Set<number>([0, ...[...this.pages.keys()].filter(n => n >= lo && n <= hi)])];
    void Promise.allSettled(want.map(n => this.api.queryFaces({ ...this.query, offset: n * PAGE_SIZE, limit: PAGE_SIZE }).then(page => ({ n, page })))).then(rs => {
      if (this.disposed || e !== this.epoch || mySeq !== this.refreshSeq) return;
      const ok = rs.flatMap(r => (r.status === 'fulfilled' ? [r.value] : []));
      const first = ok.find(x => x.n === 0);
      if (!first) { this.ensureRange(...this.lastRange); return; } // trang 0 lỗi: giữ nguyên, lần bump sau thử lại
      const total = first.page.total;
      if (total === 0 && scanning && (this.snap.total ?? 0) > 0 && !this.zeroPending) { this.zeroPending = true; this.ensureRange(...this.lastRange); return; }
      this.zeroPending = false;
      const next = new Map<number, FaceRow[]>();
      for (const { n, page } of ok) if (n === 0 || page.items.length > 0) next.set(n, page.items);
      this.pages = next; // trang lỗi lần này bị bỏ → ensureRange tải lại khi cần
      this.failed.clear();
      this.emit({ total, error: null });
      this.ensureRange(...this.lastRange);
    });
  }

  /** Lần làm tươi trước nhận total=0 khi đang quét và chưa được xác nhận. */
  hasZeroPending(): boolean { return this.zeroPending; }

  /** Thử lại các trang còn thiếu quanh vùng đang xem (sau lỗi). */
  retry() {
    if (this.disposed) return;
    this.failed.clear();
    this.emit({ error: null });
    if (this.snap.total === null && !this.inflight.has(0) && !this.pages.has(0)) this.fetchPage(0);
    this.ensureRange(...this.lastRange);
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
    const e = this.epoch; const r = this.refreshSeq;
    this.inflight.add(n);
    this.api.queryFaces({ ...this.query, offset: n * PAGE_SIZE, limit: PAGE_SIZE }).then(
      page => {
        if (this.disposed || e !== this.epoch || r !== this.refreshSeq) return; // bắt đầu trước lần làm tươi: dữ liệu cũ, bỏ
        this.inflight.delete(n);
        this.pages.set(n, page.items);
        this.emit({ total: page.total, error: null });
      },
      err => {
        if (this.disposed || e !== this.epoch || r !== this.refreshSeq) return;
        this.inflight.delete(n);
        this.failed.add(n);
        this.emit({ error: err?.message ?? String(err) });
      },
    );
  }

  private emit(patch: Partial<PagedSnapshot>) {
    this.snap = { ...this.snap, ...patch, version: this.snap.version + 1 };
    this.listeners.forEach(cb => cb());
  }
}

const EMPTY: PagedSnapshot = { total: null, version: 0, error: null, query: null, queryKey: '' };
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
