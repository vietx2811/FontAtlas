import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { mockRows } from '../../../api/mock';
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

  it('more than 4: Compare stays enabled and a narrowing hint can cut the selection to 4', () => {
    render(<TopBarActions />); pick(6);
    expect((screen.getByRole('button', { name: 'Compare' }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByText('6 font — nên ≤ 4 để so rõ.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
    expect(useSelectionStore.getState().rows.size).toBe(4);
    expect(screen.queryByText(/nên ≤ 4/)).toBeNull();
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
    fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
    expect(useSelectionStore.getState().anchor).toEqual({ key: rowKey(rows[2]), index: 2 });
  });
});
