import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { Api } from '../../../api';
import { FaceActionsProvider } from '../../faces/FaceActions';
import { NoticeStack } from '../../NoticeStack';
import { useNoticeStore } from '../../../store/notices';
import { useFavoriteStore, useSelectionStore } from '../../../store/selection';
import { useUiStore } from '../../../store/ui';
import { Library } from '../Library';

const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const initial = useUiStore.getState();
const heads = () => screen.queryAllByTestId('group-head');
const faceCells = () => screen.queryAllByRole('gridcell').filter(c => c.hasAttribute('data-face-id'));
const selectedKeys = () => [...useSelectionStore.getState().rows.keys()];
const familyOf = (key: string) => [...useSelectionStore.getState().rows.values()].find(r => `${r.hashHex}:${r.faceIndex}` === key)?.family;

async function mount(api: Api = createMockApi()) {
  const r = render(<FaceActionsProvider api={api}><NoticeStack /><Library api={api} width={1300} height={800} /></FaceActionsProvider>);
  useUiStore.getState().setGroupByFamily(true);
  await settle();
  return { api, ...r };
}

describe('Library grouped by Family: selection and collapse', () => {
  beforeEach(() => {
    useUiStore.setState(initial, true); useSelectionStore.getState().clear();
    useFavoriteStore.setState({ overrides: new Map() }); useNoticeStore.setState({ notices: [] });
  });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it('collapsing a group deselects its fonts (they are not visible) and says how many; fonts of other groups stay selected', async () => {
    await mount();
    expect(heads().length).toBeGreaterThan(1);
    const second = heads()[1].dataset.family!;
    fireEvent.click(heads()[0]); // đóng nhóm 1 để chắc nhóm 2 ở lại; rồi chọn tất cả
    await settle();
    fireEvent.click(heads()[0]); await settle(); // mở lại
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', metaKey: true, bubbles: true, cancelable: true })); await settle();
    const total = selectedKeys().length;
    const inFirst = selectedKeys().filter(k => familyOf(k) === heads()[0].dataset.family).length;
    expect(inFirst).toBeGreaterThan(0);
    fireEvent.click(heads()[0]); await settle();
    expect(selectedKeys().length).toBe(total - inFirst);
    expect(selectedKeys().some(k => familyOf(k) === heads()[0].dataset.family)).toBe(false);
    expect(selectedKeys().some(k => familyOf(k) === second)).toBe(true);
    expect(useNoticeStore.getState().notices.map(n => n.text).join(' ')).toMatch(new RegExp(`Đã bỏ chọn ${inFirst} font trong nhóm thu gọn`));
  });

  it('⌘A selects only the visible fonts (collapsed groups skipped) and says so; collapsing nothing selects everything', async () => {
    const { api } = await mount();
    const all = (await api.queryFaces({ limit: 1 })).total;
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', metaKey: true, bubbles: true, cancelable: true })); await settle();
    expect(selectedKeys()).toHaveLength(all);
    useSelectionStore.getState().clear();
    fireEvent.click(heads()[0]); await settle();
    const hiddenN = (await api.familyGroups({ groupByFamily: true }))[0].count;
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', metaKey: true, bubbles: true, cancelable: true })); await settle();
    expect(selectedKeys()).toHaveLength(all - hiddenN);
    expect(useNoticeStore.getState().notices.map(n => n.text).join(' ')).toMatch(new RegExp(`bỏ qua ${hiddenN} font trong nhóm thu gọn`));
  });

  it('the collapsed state lives in the store: it survives the grid being remounted (rescan) and is cleared when the query changes', async () => {
    const { api, unmount } = await mount();
    const fam = heads()[0].dataset.family!;
    fireEvent.click(heads()[0]); await settle();
    expect(useUiStore.getState().groupCollapsed.has(fam)).toBe(true);
    unmount();
    render(<FaceActionsProvider api={api}><Library api={api} width={1300} height={800} /></FaceActionsProvider>); await settle();
    expect(heads()[0].getAttribute('aria-expanded')).toBe('false');
    act(() => useUiStore.getState().setQuery('a')); await settle();
    expect(useUiStore.getState().groupCollapsed.size).toBe(0);
    void faceCells;
  });
});
