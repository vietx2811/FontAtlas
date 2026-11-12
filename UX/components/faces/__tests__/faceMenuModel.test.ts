import { describe, expect, it, vi } from 'vitest';
import { makeRow } from '../../../features/library-grid/api';
import type { MenuEntry } from '../../ui/Menu';
import { buildFaceMenu, projectEntries, type FaceMenuActions, type FaceMenuCtx } from '../faceMenuModel';

const actions = (): FaceMenuActions => ({
  openDetail: vi.fn(), viewFamily: vi.fn(), findSimilar: vi.fn(), pairWith: vi.fn(), setFavorite: vi.fn(), addToProject: vi.fn(), newProject: vi.fn(),
  tag: vi.fn(), newTag: vi.fn(), setActive: vi.fn(), reveal: vi.fn(), copyNames: vi.fn(), removeFromProject: vi.fn(),
});
const ctx = (over: Partial<FaceMenuCtx> = {}): FaceMenuCtx => ({
  view: 'all', isFavorite: () => false, isActive: () => false,
  collections: [{ id: 7, name: 'Menu quán', color: null, itemCount: 3 }], tags: [{ id: 2, name: 'Serif', faceCount: 5 }], actions: actions(), ...over,
});
const ids = (e: MenuEntry[]) => e.flatMap(x => ('id' in x && x.id && x.type !== 'sep' ? [x.id] : []));
const find = (e: MenuEntry[], id: string) => e.find(x => 'id' in x && x.id === id) as Extract<MenuEntry, { label: string }>;

describe('buildFaceMenu', () => {
  it('one font: navigation, favourite, project, tag, activate, Finder, copy', () => {
    const e = buildFaceMenu([makeRow(1)], ctx());
    expect(ids(e)).toEqual(['open', 'family', 'similar', 'pair', 'fav', 'project', 'tag', 'activate', 'reveal', 'copy']);
    expect(find(e, 'reveal').disabled).toBe(false);
  });

  it('many fonts: no per-font navigation; header counts; Finder disabled with a reason', () => {
    const e = buildFaceMenu([makeRow(1), makeRow(2), makeRow(3)], ctx());
    expect(ids(e)).toEqual(['count', 'fav', 'project', 'tag', 'activate', 'reveal', 'copy']);
    expect((e[0] as { text: string }).text).toBe('3 font đã chọn');
    const reveal = find(e, 'reveal');
    expect(reveal.disabled).toBe(true);
    expect(reveal.reason).toMatch(/một font/);
  });

  it('favourite label follows the whole selection', () => {
    const rows = [makeRow(1), makeRow(2)];
    expect(find(buildFaceMenu(rows, ctx({ isFavorite: r => r.faceId === 2 })), 'fav').label).toBe('Yêu thích');
    const all = buildFaceMenu(rows, ctx({ isFavorite: () => true }));
    expect(find(all, 'fav').label).toBe('Bỏ yêu thích');
    const c = ctx({ isFavorite: () => true });
    find(buildFaceMenu(rows, c), 'fav').onSelect!();
    expect(c.actions.setFavorite).toHaveBeenCalledWith(rows, false);
  });

  it('activate label and payload: skips missing files; disabled with reason when none usable', () => {
    const ok = makeRow(1); const gone = makeRow(2, { missing: true });
    const c = ctx();
    const e = buildFaceMenu([ok, gone], c);
    expect(find(e, 'activate').label).toBe('Kích hoạt');
    find(e, 'activate').onSelect!();
    expect(c.actions.setActive).toHaveBeenCalledWith([ok], true);
    expect(find(buildFaceMenu([ok], ctx({ isActive: () => true })), 'activate').label).toBe('Tắt kích hoạt');
    const none = find(buildFaceMenu([gone], ctx()), 'activate');
    expect(none.disabled).toBe(true); expect(none.reason).toBeTruthy();
  });

  it('project submenu lists projects then "Dự án mới…"; loading state while null', () => {
    const rows = [makeRow(1), makeRow(2)];
    const c = ctx();
    const sub = find(buildFaceMenu(rows, c), 'project').items!;
    expect(ids(sub)).toEqual(['col-7', 'new-project']);
    (sub[0] as { onSelect(): void }).onSelect();
    expect(c.actions.addToProject).toHaveBeenCalledWith(rows, 7, 'Menu quán');
    find(sub, 'new-project').onSelect!();
    expect(c.actions.newProject).toHaveBeenCalledWith(rows);
    expect(ids(projectEntries(rows, ctx({ collections: null })))).toEqual(['loading', 'new-project']);
  });

  it('"Gỡ khỏi dự án" only inside a project view, and the current project cannot be a target', () => {
    expect(ids(buildFaceMenu([makeRow(1)], ctx()))).not.toContain('unproject');
    const c = ctx({ view: 'col:7' });
    const e = buildFaceMenu([makeRow(1)], c);
    find(e, 'unproject').onSelect!();
    expect(c.actions.removeFromProject).toHaveBeenCalledWith([expect.objectContaining({ faceId: 2 })], 7);
    expect(find(find(e, 'project').items!, 'col-7').disabled).toBe(true);
  });

  it('tag submenu: existing tags then "Thẻ mới…"', () => {
    const c = ctx();
    const sub = find(buildFaceMenu([makeRow(1)], c), 'tag').items!;
    expect(ids(sub)).toEqual(['tag-2', 'new-tag']);
    (sub[0] as { onSelect(): void }).onSelect();
    expect(c.actions.tag).toHaveBeenCalledWith([expect.objectContaining({ faceId: 2 })], 2, 'Serif');
  });
});

describe('empty lists', () => {
  it('no stray separator above "Dự án mới…" / "Thẻ mới…" when there is nothing to list', () => {
    const e = buildFaceMenu([makeRow(1)], ctx({ collections: [], tags: [] }));
    expect(find(e, 'project').items!.map(x => x.type ?? 'item')).toEqual(['item']);
    expect(find(e, 'tag').items!.map(x => x.type ?? 'item')).toEqual(['item']);
  });
});

describe('Thêm vào… liệt kê Case (G1)', () => {
  const rows = [makeRow(1)];
  it('có Case thì thêm nhóm Case sau "Dự án mới…"; chọn một Case gọi addToCase; không có addToCase hoặc không có Case thì không có nhóm', () => {
    const addToCase = vi.fn(); const base = actions();
    const withCase = projectEntries(rows, ctx({ cases: [{ id: 3, name: 'Poster', faceCount: 4 }], actions: { ...base, addToCase } }));
    const ids = withCase.map(e => (e as { id?: string }).id);
    expect(ids.slice(-3)).toEqual(['sep-case', 'lbl-case', 'case-3']);
    (withCase.at(-1) as { onSelect(): void }).onSelect(); expect(addToCase).toHaveBeenCalledWith(rows, 3, 'Poster');
    expect(projectEntries(rows, ctx({ cases: [{ id: 3, name: 'Poster', faceCount: 4 }] })).some(e => (e as { id?: string }).id === 'case-3')).toBe(false);
    expect(projectEntries(rows, ctx({ cases: [], actions: { ...base, addToCase } })).some(e => (e as { id?: string }).id === 'lbl-case')).toBe(false);
  });
});
