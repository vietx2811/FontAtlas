import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { rowKey, useSelectionStore } from '../../../store/selection';
import { useUiStore } from '../../../store/ui';
import { TopBarActions } from '../TopBarActions';
import { tipOf } from '../../../test/tip';

const rows = mockRows();
const pick = (n: number) => act(() => useSelectionStore.getState().replace(rows.slice(0, n), n ? { key: rowKey(rows[0]), index: 0 } : null));
const ui0 = useUiStore.getState();
const names = () => screen.queryAllByRole('button').map(b => b.getAttribute('aria-label') ?? b.textContent);
beforeEach(() => { useUiStore.setState(ui0, true); useSelectionStore.getState().clear(); useNoticeStore.setState({ notices: [] }); });

describe('TopBarActions', () => {
  it('shows nothing with no selection', () => {
    render(<TopBarActions />);
    expect(screen.queryByRole('group', { name: 'Pair / Compare' })).toBeNull();
  });

  it('1 font: Pair and Find similar; Pair opens the Pair tab on that font', () => {
    render(<TopBarActions />); pick(1);
    expect(screen.getByRole('button', { name: 'Pair' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Find similar' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Compare' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Pair' }));
    expect(useUiStore.getState()).toMatchObject({ selectedId: rowKey(rows[0]), panelTab: 'pair' });
    fireEvent.click(screen.getByRole('button', { name: 'Find similar' }));
    expect(useUiStore.getState().panelTab).toBe('sim');
  });

  it('2–4 fonts: Compare and Pair these, with the selection count', () => {
    render(<TopBarActions />); pick(3);
    expect(screen.getByText('3 chọn')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Compare' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Pair these' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Find similar' })).toBeNull();
    expect(screen.queryByText(/nên ≤ 4/)).toBeNull();
  });

  it('Compare opens the Compare screen with the selected faces, first one as the base', () => {
    render(<TopBarActions />); pick(3);
    fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
    expect(useUiStore.getState().compare).toEqual(rows.slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    expect(useNoticeStore.getState().notices).toHaveLength(0);
  });

  it('more than 4: Compare stays enabled; a floating tag (not a layout row) offers cutting the selection to 4', () => {
    const { container } = render(<TopBarActions />); pick(6);
    expect((screen.getByRole('button', { name: 'Compare' }) as HTMLButtonElement).disabled).toBe(false);
    const tag = screen.getByTestId('narrow-tag');
    expect(tag.textContent).toBe('!6 · nên ≤ 4');
    expect(tag.closest('.fa-act-wrap')?.querySelector('button[aria-label="Compare"]')).toBeTruthy(); // gắn ngay nút Compare
    expect(container.querySelector('.fa-act-hint, [role=status]')).toBeNull(); // không còn dòng inline
    expect(screen.queryByTestId('narrow-pop')).toBeNull();
    fireEvent.focus(tag);
    expect(screen.getByTestId('narrow-pop').textContent).toContain('6 font đang chọn — nên ≤ 4 để so cho rõ.');
    fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
    expect(useSelectionStore.getState().rows.size).toBe(4);
    expect(screen.queryByTestId('narrow-tag')).toBeNull();
  });

  it('the narrowing popover opens on hover, survives moving onto it, and closes on Esc', async () => {
    render(<TopBarActions />); pick(5);
    const tag = screen.getByTestId('narrow-tag');
    fireEvent.pointerEnter(tag.parentElement!);
    expect(screen.getByTestId('narrow-pop')).toBeTruthy();
    fireEvent.pointerLeave(tag.parentElement!);
    fireEvent.pointerEnter(tag.parentElement!); // vào lại trong 120 ms: không đóng
    await act(async () => { await new Promise(r => setTimeout(r, 200)); });
    expect(screen.getByTestId('narrow-pop')).toBeTruthy();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.queryByTestId('narrow-pop')).toBeNull();
  });

  it('the tag floats: CSS positions it absolutely so it takes no layout row', () => {
    const css = readFileSync(join(process.cwd(), 'components/main/topbarActions.css'), 'utf8');
    expect(css).toMatch(/\.fa-narrow\s*\{[^}]*position:\s*absolute/);
    expect(css).not.toMatch(/fa-act-hint/);
  });

  it('the count chip clears the selection', () => {
    render(<TopBarActions />); pick(2);
    fireEvent.click(screen.getByRole('button', { name: 'Bỏ chọn' }));
    expect(useSelectionStore.getState().rows.size).toBe(0);
  });

  it('⇧P and ⇧C trigger Pair / Compare; ignored while typing in a field', () => {
    render(<><input aria-label="x" /><TopBarActions /></>); pick(2);
    fireEvent.keyDown(document.body, { key: 'P', shiftKey: true });
    expect(useUiStore.getState().pairThese).toHaveLength(2);
    useUiStore.getState().closePairThese();
    fireEvent.keyDown(document.body, { key: 'C', shiftKey: true });
    expect(useUiStore.getState().compare).toHaveLength(2);
    useUiStore.getState().setPanelTab('spec');
    const input = screen.getByLabelText('x'); input.focus();
    fireEvent.keyDown(input, { key: 'P', shiftKey: true });
    expect(useUiStore.getState().panelTab).toBe('spec');
  });

  it('⇧C does nothing for a single font (Compare needs 2+)', () => {
    render(<TopBarActions />); pick(1);
    fireEvent.keyDown(document.body, { key: 'C', shiftKey: true });
    expect(useUiStore.getState().compare).toBeNull();
  });

  it('buttons carry the shortcut in their tooltip', async () => {
    render(<TopBarActions />); pick(2);
    expect(await tipOf(screen.getByRole('button', { name: 'Compare' }))).toContain('⇧C');
    expect(names().length).toBeGreaterThan(2);
  });

  it('Pair these (2+) opens the Pair these screen with the selected faces; single Pair opens the Pair tab quietly', () => {
    render(<TopBarActions />); pick(1);
    fireEvent.click(screen.getByRole('button', { name: 'Pair' }));
    expect(useNoticeStore.getState().notices).toHaveLength(0);
    expect(useUiStore.getState()).toMatchObject({ panelTab: 'pair', pairThese: null });
    pick(3);
    fireEvent.click(screen.getByRole('button', { name: 'Pair these' }));
    expect(useUiStore.getState().pairThese).toEqual(rows.slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    expect(useNoticeStore.getState().notices).toHaveLength(0);
  });

  it('Pair these takes at most four fonts and says so', () => {
    render(<TopBarActions />); pick(6);
    fireEvent.click(screen.getByRole('button', { name: 'Pair these' }));
    expect(useUiStore.getState().pairThese).toHaveLength(4);
    expect(useNoticeStore.getState().notices[0].text).toMatch(/tối đa 4/);
  });

  it('shortcuts are ignored under menus, dialogs, repeats, IME and prevented events', () => {
    render(<TopBarActions />); pick(2);
    const attempt = (init: KeyboardEventInit = {}) => { useUiStore.getState().closePairThese(); fireEvent.keyDown(document.body, { key: 'P', shiftKey: true, ...init }); return useUiStore.getState().pairThese ? 'pair' : 'spec'; };
    for (const role of ['menu', 'dialog', 'alertdialog']) {
      const el = document.createElement('div'); el.setAttribute('role', role); document.body.append(el);
      expect(attempt()).toBe('spec'); el.remove();
    }
    const modal = document.createElement('div'); modal.setAttribute('aria-modal', 'true'); document.body.append(modal);
    expect(attempt()).toBe('spec'); modal.remove();
    expect(attempt({ repeat: true })).toBe('spec');
    expect(attempt({ isComposing: true })).toBe('spec');
    expect(attempt()).toBe('pair');
  });

  it('narrowing keeps the anchor when it is still inside the first four', () => {
    render(<TopBarActions />);
    act(() => useSelectionStore.getState().replace(rows.slice(0, 6), { key: rowKey(rows[2]), index: 2 }));
    fireEvent.focus(screen.getByTestId('narrow-tag'));
    fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
    expect(useSelectionStore.getState().anchor).toEqual({ key: rowKey(rows[2]), index: 2 });
  });

  describe('Activate / Deactivate (⌘⇧A)', () => {
    const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
    const stateOf = async (api: ReturnType<typeof createMockApi>) => (await api.activationState()).map(s => s.hashHex).sort();

    const keys = () => { fireEvent.keyDown(document.body, { key: 'A', metaKey: true, shiftKey: true }); };

    it('2+ fonts: no activation button on the Top Bar (the selection bar owns it); ⌘⇧A turns the whole selection on, then off', async () => {
      const api = createMockApi();
      render(<TopBarActions api={api} />); pick(3);
      expect(screen.queryByRole('button', { name: /^(Activate|Deactivate)$/ })).toBeNull();
      expect(screen.queryByTestId('topbar-activate')).toBeNull();
      keys(); await settle();
      expect(await stateOf(api)).toEqual(rows.slice(0, 3).map(r => r.hashHex).sort());
      keys(); await settle();
      expect(await stateOf(api)).toEqual([]);
    });

    it('a partly active selection: ⌘⇧A turns the rest on', async () => {
      const api = createMockApi();
      await api.activationActivate([{ hashHex: rows[0].hashHex, faceIndex: rows[0].faceIndex }]);
      await act(async () => { await useUiStore.getState().refreshActive(api); });
      render(<TopBarActions api={api} />); pick(2);
      keys(); await settle();
      expect(await stateOf(api)).toEqual(rows.slice(0, 2).map(r => r.hashHex).sort());
    });

    it('⌘⇧A toggles the selection, also with a single font (no button there)', async () => {
      const api = createMockApi();
      render(<TopBarActions api={api} />); pick(1);
      expect(screen.queryByRole('button', { name: 'Activate' })).toBeNull();
      keys(); await settle();
      expect(await stateOf(api)).toEqual([rows[0].hashHex]);
      fireEvent.keyDown(document.body, { key: 'A', metaKey: true, shiftKey: true }); await settle();
      expect(await stateOf(api)).toEqual([]);
    });

    it('⌘⇧A is ignored while typing in a field, and plain ⌘A / ⇧A do nothing here', async () => {
      const api = createMockApi();
      render(<><input aria-label="q" /><TopBarActions api={api} /></>); pick(2);
      const input = screen.getByLabelText('q');
      fireEvent.keyDown(input, { key: 'A', metaKey: true, shiftKey: true }); await settle();
      fireEvent.keyDown(document.body, { key: 'a', metaKey: true }); fireEvent.keyDown(document.body, { key: 'A', shiftKey: true }); await settle();
      expect(await stateOf(api)).toEqual([]);
    });

    it('without an api there is no activation button', () => {
      render(<TopBarActions />); pick(3);
      expect(screen.queryByTestId('topbar-activate')).toBeNull();
    });
  });
});

describe('NarrowTag focus and description', () => {
  it('describes itself with real text and closes at once when focus leaves (no timer race)', () => {
    render(<TopBarActions />); 
    act(() => useSelectionStore.getState().replace(rows.slice(0, 6), { key: rowKey(rows[0]), index: 0 }));
    const tag = screen.getByTestId('narrow-tag');
    const desc = document.getElementById(tag.getAttribute('aria-describedby')!)!;
    expect(desc.textContent).toMatch(/6/);
    expect(desc.textContent!.length).toBeGreaterThan(10);
    fireEvent.focus(tag);
    expect(screen.getByTestId('narrow-pop')).toBeTruthy();
    fireEvent.blur(tag, { relatedTarget: document.body });
    expect(screen.queryByTestId('narrow-pop')).toBeNull();
  });

  it('after "Chỉ lấy 4 đầu" focus moves to the Compare button instead of falling to <body>', async () => {
    vi.useFakeTimers();
    try {
      render(<TopBarActions />);
      act(() => useSelectionStore.getState().replace(rows.slice(0, 6), { key: rowKey(rows[0]), index: 0 }));
      fireEvent.focus(screen.getByTestId('narrow-tag'));
      fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
      await act(async () => { await vi.advanceTimersByTimeAsync(10); });
      expect(screen.queryByTestId('narrow-tag')).toBeNull();
      expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Compare' }));
    } finally { vi.useRealTimers(); }
  });

  it('Esc is left to an open dialog; with none open it closes the popover', () => {
    render(<TopBarActions />);
    act(() => useSelectionStore.getState().replace(rows.slice(0, 6), { key: rowKey(rows[0]), index: 0 }));
    fireEvent.focus(screen.getByTestId('narrow-tag'));
    const dlg = document.createElement('div'); dlg.setAttribute('role', 'dialog'); dlg.setAttribute('aria-modal', 'true'); document.body.appendChild(dlg);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.getByTestId('narrow-pop')).toBeTruthy();
    dlg.remove();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.queryByTestId('narrow-pop')).toBeNull();
  });
});
