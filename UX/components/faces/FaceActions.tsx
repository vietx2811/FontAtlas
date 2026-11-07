import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { Api, CollectionDto, FaceKeyDto, TagDto } from '../../api';
import type { FaceRow } from '../../features/library-grid/types';
import { useNoticeStore } from '../../store/notices';
import { dropFromSelection, rowKey, useFavoriteStore, useSelectionStore } from '../../store/selection';
import { faceKey, useUiStore } from '../../store/ui';
import { useActivationFlow } from '../activation/useActivationFlow';
import { Menu, type MenuAnchor } from '../ui/Menu';
import { NamePrompt } from '../ui/NamePrompt';
import { buildFaceMenu, faceName, projectEntries, type FaceMenuActions } from './faceMenuModel';
import { fetchFamilyScope } from './familyScope';

export type OpenOpts = { keyboard?: boolean; returnFocus?: HTMLElement | null };
export interface FaceActionsApi {
  /** Menu ngữ cảnh đầy đủ cho 1 hoặc nhiều font. */
  openMenu(rows: readonly FaceRow[], at: MenuAnchor, opts?: OpenOpts): void;
  /** Chỉ danh sách dự án (nút "Thêm vào dự án" của thanh chọn). */
  openProjectMenu(rows: readonly FaceRow[], at: MenuAnchor, opts?: OpenOpts): void;
  setFavorite(rows: readonly FaceRow[], on: boolean): Promise<void>;
  setActive(rows: readonly FaceRow[], on: boolean): Promise<void>;
  isFavorite(row: FaceRow): boolean;
}

const Ctx = createContext<FaceActionsApi | null>(null);
/** Null khi không có provider (test riêng lẻ): nơi gọi bỏ qua và để menu mặc định của trình duyệt. */
export const useFaceActions = (): FaceActionsApi | null => useContext(Ctx);

const CHUNK = 12;
const keyOf = (r: FaceRow): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const what = (rows: readonly FaceRow[]): string => (rows.length === 1 ? `«${faceName(rows[0])}»` : `${rows.length.toLocaleString('vi-VN')} font`);
const errText = (e: unknown): string => (e as { message?: string } | null)?.message ?? String(e);
const say = (kind: 'info' | 'error', text: string): void => void useNoticeStore.getState().push({ kind, text });

async function copyText(text: string): Promise<void> {
  try { await navigator.clipboard.writeText(text); return; } catch { /* WebView không cho: thử cách cũ */ }
  const ta = document.createElement('textarea');
  ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0';
  document.body.appendChild(ta); ta.select();
  const ok = document.execCommand('copy');
  ta.remove();
  if (!ok) throw new Error('Không ghi được vào clipboard');
}

type MenuState = { kind: 'full' | 'projects'; rows: readonly FaceRow[]; anchor: MenuAnchor; opts: OpenOpts };
type PromptState = { kind: 'project' | 'tag'; rows: readonly FaceRow[]; anchor: MenuAnchor };

export function FaceActionsProvider({ api, children }: { api: Api; children: ReactNode }): React.JSX.Element {
  const flow = useActivationFlow(api);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [prompt, setPrompt] = useState<PromptState | null>(null);
  const [data, setData] = useState<{ cols: CollectionDto[]; tags: TagDto[] } | null>(null);
  const active = useUiStore(s => s.active);
  const flowRef = useRef(flow); flowRef.current = flow;
  const anchorRef = useRef<MenuAnchor>({ x: 0, y: 0 });

  // Đọc store tại thời điểm gọi → `value` của context ổn định, không làm cả lưới vẽ lại mỗi lần đổi yêu thích.
  const isFavorite = useCallback((r: FaceRow) => useFavoriteStore.getState().overrides.get(r.faceId) ?? r.isFavorite, []);

  // Mỗi lần mở menu: nạp danh sách dự án/thẻ mới nhất (có thể vừa đổi ở thanh bên).
  const menuRows = menu?.rows;
  useEffect(() => {
    if (!menuRows) return;
    let alive = true;
    setData(null);
    Promise.all([api.listCollections(), api.listTags()]).then(
      ([cols, tags]) => { if (alive) setData({ cols, tags }); },
      e => { if (alive) { setData({ cols: [], tags: [] }); say('error', `Không tải được danh sách dự án/thẻ: ${errText(e)}`); } },
    );
    return () => { alive = false; };
  }, [menuRows, api]);

  const setFavorite = useCallback(async (rows: readonly FaceRow[], on: boolean) => {
    const todo = rows.filter(r => isFavorite(r) !== on);
    if (!todo.length) return;
    const done: FaceRow[] = []; const failed: string[] = [];
    for (let i = 0; i < todo.length; i += CHUNK) {
      const part = todo.slice(i, i + CHUNK);
      const res = await Promise.allSettled(part.map(r => api.setFavorite(keyOf(r), on)));
      res.forEach((r, j) => (r.status === 'fulfilled' ? done.push(part[j]) : failed.push(errText(r.reason))));
    }
    if (done.length) {
      useFavoriteStore.getState().set(done, on);
      say('info', on ? `Đã thêm ${what(done)} vào Yêu thích` : `Đã bỏ ${what(done)} khỏi Yêu thích`);
      const reload = !on && useUiStore.getState().view === 'star';
      if (reload) dropFromSelection(done); // lưới sắp nạp lại và các thẻ này không còn trong truy vấn
      useUiStore.getState().bumpLibrary(reload);
    }
    if (failed.length) say('error', `${failed.length} font không đổi được yêu thích: ${failed[0]}`);
  }, [api, isFavorite]);

  const setActive = useCallback(async (rows: readonly FaceRow[], on: boolean) => {
    const faces = rows.filter(r => !r.missing).map(keyOf);
    if (faces.length) await flowRef.current.toggle(faces, on);
  }, []);

  const addToProject = useCallback(async (rows: readonly FaceRow[], id: number, name: string) => {
    try {
      await api.addToCollection(id, rows.map(keyOf));
      say('info', `Đã thêm ${what(rows)} vào «${name}»`);
      useUiStore.getState().bumpLibrary(false);
    } catch (e) { say('error', `Không thêm được vào «${name}»: ${errText(e)}`); }
  }, [api]);

  const actions: FaceMenuActions = useMemo(() => ({
    openDetail: r => { useUiStore.getState().select(faceKey(r)); },
    viewFamily: r => {
      fetchFamilyScope(api, r.family).then(
        scope => useUiStore.getState().openFamily(scope),
        e => say('error', errText(e)),
      );
    },
    findSimilar: r => { const ui = useUiStore.getState(); ui.select(faceKey(r)); ui.setPanelTab('sim'); },
    pairWith: r => { const ui = useUiStore.getState(); ui.select(faceKey(r)); ui.setPanelTab('pair'); },
    setFavorite: (rows, on) => void setFavorite(rows, on),
    addToProject: (rows, id, name) => void addToProject(rows, id, name),
    newProject: rows => setPrompt({ kind: 'project', rows, anchor: anchorRef.current }),
    tag: (rows, id, name) => {
      api.tagFaces(id, rows.map(keyOf)).then(
        () => { say('info', `Đã gắn thẻ «${name}» cho ${what(rows)}`); useUiStore.getState().bumpLibrary(false); },
        e => say('error', `Không gắn được thẻ «${name}»: ${errText(e)}`),
      );
    },
    newTag: rows => setPrompt({ kind: 'tag', rows, anchor: anchorRef.current }),
    setActive: (rows, on) => void setActive(rows, on),
    reveal: r => { api.revealInFinder(keyOf(r)).catch(e => say('error', `Không mở được Finder: ${errText(e)}`)); },
    copyNames: rows => {
      copyText(rows.map(faceName).join('\n')).then(
        () => say('info', rows.length === 1 ? 'Đã sao chép tên font' : `Đã sao chép tên ${rows.length.toLocaleString('vi-VN')} font`),
        e => say('error', errText(e)),
      );
    },
    removeFromProject: (rows, id) => {
      api.removeFromCollection(id, rows.map(keyOf)).then(
        () => { say('info', `Đã gỡ ${what(rows)} khỏi dự án`); useSelectionStore.getState().clear(); useUiStore.getState().bumpLibrary(true); },
        e => say('error', `Không gỡ được khỏi dự án: ${errText(e)}`),
      );
    },
  }), [api, setFavorite, setActive, addToProject]);

  const open = useCallback((kind: MenuState['kind'], rows: readonly FaceRow[], anchor: MenuAnchor, opts: OpenOpts = {}) => {
    if (!rows.length) return;
    anchorRef.current = anchor;
    setPrompt(null);
    setMenu({ kind, rows, anchor, opts });
  }, []);

  const value = useMemo<FaceActionsApi>(() => ({
    openMenu: (rows, at, opts) => open('full', rows, at, opts),
    openProjectMenu: (rows, at, opts) => open('projects', rows, at, opts),
    setFavorite, setActive, isFavorite,
  }), [open, setFavorite, setActive, isFavorite]);

  const submitPrompt = async (name: string) => {
    const p = prompt!; setPrompt(null);
    try {
      if (p.kind === 'project') {
        const c = await api.createCollection(name, null);
        await addToProject(p.rows, c.id, c.name);
      } else {
        const t = await api.createTag(name);
        await api.tagFaces(t.id, p.rows.map(keyOf));
        say('info', `Đã gắn thẻ «${t.name}» cho ${what(p.rows)}`);
        useUiStore.getState().bumpLibrary(false);
      }
    } catch (e) { say('error', `${p.kind === 'project' ? 'Không tạo được dự án' : 'Không tạo được thẻ'}: ${errText(e)}`); }
  };

  const entries = menu && (() => {
    const ctx = {
      view: useUiStore.getState().view, isFavorite, isActive: (r: FaceRow) => active.has(r.hashHex),
      collections: data?.cols ?? null, tags: data?.tags ?? null, actions,
    };
    return menu.kind === 'projects' ? projectEntries(menu.rows, ctx) : buildFaceMenu(menu.rows, ctx);
  })();

  return (
    <Ctx.Provider value={value}>
      {children}
      {flow.dialog}
      {menu && entries && (
        <Menu key={`${menu.kind}:${menu.rows.map(rowKey).join('|')}`} anchor={menu.anchor} focusFirst={menu.opts.keyboard} returnFocus={menu.opts.returnFocus}
          label={menu.kind === 'projects' ? 'Thêm vào dự án' : 'Menu font'} onClose={() => setMenu(null)} entries={entries} />
      )}
      {prompt && <NamePrompt label={prompt.kind === 'project' ? 'Tên dự án' : 'Tên thẻ'} initial={prompt.kind === 'project' ? 'Dự án mới' : ''} anchor={prompt.anchor}
        onSubmit={n => void submitPrompt(n)} onCancel={() => setPrompt(null)} />}
    </Ctx.Provider>
  );
}

/** onContextMenu + phím ContextMenu/Shift+F10 cho một khối chứa font (kết quả Tương tự, ứng viên Ghép cặp…). Không có provider → rỗng. */
export function faceMenuProps(actions: FaceActionsApi | null, rows: readonly FaceRow[]): {
  onContextMenu?(e: React.MouseEvent<HTMLElement>): void; onKeyDown?(e: React.KeyboardEvent<HTMLElement>): void;
} {
  if (!actions) return {};
  return {
    onContextMenu: e => { e.preventDefault(); actions.openMenu(rows, { x: e.clientX, y: e.clientY }, { returnFocus: e.target instanceof HTMLElement ? e.target : null }); },
    onKeyDown: e => {
      if (e.key !== 'ContextMenu' && !(e.key === 'F10' && e.shiftKey)) return;
      e.preventDefault();
      const el = e.target as HTMLElement; const r = el.getBoundingClientRect();
      actions.openMenu(rows, { x: r.left + 8, y: r.bottom }, { keyboard: true, returnFocus: el });
    },
  };
}
