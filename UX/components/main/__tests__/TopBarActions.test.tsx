import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
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
    render(<TopBarActions api={createMockApi()} />);
    expect(screen.queryByRole('group', { name: 'Pair / Compare' })).toBeNull();
  });

  it('1 font: Pair and Find similar; Pair opens the Pair tab on that font', () => {
    render(<TopBarActions api={createMockApi()} />); pick(1);
    expect(screen.getByRole('button', { name: 'Pair' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Find similar' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Compare' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Pair' }));
    expect(useUiStore.getState()).toMatchObject({ selectedId: rowKey(rows[0]), panelTab: 'pair' });
    fireEvent.click(screen.getByRole('button', { name: 'Find similar' }));
    expect(useUiStore.getState().panelTab).toBe('sim');
  });

  it('2–4 fonts: Compare and Pair these, with the selection count', () => {
    render(<TopBarActions api={createMockApi()} />); pick(3);
    expect(screen.getByText('3 chọn')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Compare' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Pair these' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Find similar' })).toBeNull();
    expect(screen.queryByText(/nên ≤ 4/)).toBeNull();
  });

  it('Compare has no backend yet: it opens the nearest screen (first font, Anatomy) and says so', () => {
    render(<TopBarActions api={createMockApi()} />); pick(2);
    fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
    expect(useUiStore.getState()).toMatchObject({ selectedId: rowKey(rows[0]), panelTab: 'anat' });
    expect(useNoticeStore.getState().notices[0].text).toMatch(/Compare/);
  });

  it('more than 4: Compare stays enabled and a narrowing hint can cut the selection to 4', () => {
    render(<TopBarActions api={createMockApi()} />); pick(6);
    expect((screen.getByRole('button', { name: 'Compare' }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByText('6 font — nên ≤ 4 để so rõ.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Chỉ lấy 4 đầu' }));
    expect(useSelectionStore.getState().rows.size).toBe(4);
    expect(screen.queryByText(/nên ≤ 4/)).toBeNull();
  });

  it('the count chip clears the selection', () => {
    render(<TopBarActions api={createMockApi()} />); pick(2);
    fireEvent.click(screen.getByRole('button', { name: 'Bỏ chọn' }));
    expect(useSelectionStore.getState().rows.size).toBe(0);
  });

  it('⇧P and ⇧C trigger Pair / Compare; ignored while typing in a field', () => {
    render(<><input aria-label="x" /><TopBarActions api={createMockApi()} /></>); pick(2);
    fireEvent.keyDown(document.body, { key: 'P', shiftKey: true });
    expect(useUiStore.getState().panelTab).toBe('pair');
    fireEvent.keyDown(document.body, { key: 'C', shiftKey: true });
    expect(useUiStore.getState().panelTab).toBe('anat');
    useUiStore.getState().setPanelTab('spec');
    const input = screen.getByLabelText('x'); input.focus();
    fireEvent.keyDown(input, { key: 'P', shiftKey: true });
    expect(useUiStore.getState().panelTab).toBe('spec');
  });

  it('⇧C does nothing for a single font (Compare needs 2+)', () => {
    render(<TopBarActions api={createMockApi()} />); pick(1);
    fireEvent.keyDown(document.body, { key: 'C', shiftKey: true });
    expect(useUiStore.getState().panelTab).not.toBe('anat');
  });

  it('buttons carry the shortcut in their tooltip', async () => {
    render(<TopBarActions api={createMockApi()} />); pick(2);
    expect(await tipOf(screen.getByRole('button', { name: 'Compare' }))).toContain('⇧C');
    expect(names().length).toBeGreaterThan(2);
  });
});
