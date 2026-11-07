import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import type { ClassificationDto } from '../../../api';
import { tipOf } from '../../../test/tip';
import { ClassChip } from '../ClassChip';

const r0 = mockRows()[0];
const face = { hashHex: r0.hashHex, faceIndex: r0.faceIndex };
const info = (o: Partial<ClassificationDto> = {}): ClassificationDto => ({ class: 'serif', label: 'Serif', subclass: 'didone', confidence: 0.94, candidate: null, traits: [], signals: [{ source: 'measure', detail: 'contrast 4,89', vote: 'serif', weight: 0.7 }, { source: 'panose', detail: 'serif style 11', vote: 'serif', weight: 0.4 }], usage: ['display'], source: 'auto', autoClass: null, ...o });
const mount = (i: ClassificationDto, api = createMockApi(), onChanged = vi.fn()) => { render(<ClassChip api={api} face={face} info={i} onChanged={onChanged} />); return { api, onChanged }; };

describe('ClassChip', () => {
  it('shows class and subclass as English terms; Display is never shown as the class', () => {
    mount(info());
    const chip = screen.getByTestId('class-chip');
    expect(chip.textContent).toContain('Serif');
    expect(chip.textContent).toContain('Didone');
    expect(chip.textContent).not.toMatch(/Display/);
  });

  it('tooltip lists confidence, signals and usage (Display lives in usage)', async () => {
    mount(info());
    const tip = await tipOf(screen.getByTestId('class-chip'));
    expect(tip).toContain('94%');
    expect(tip).toContain('contrast 4,89');
    expect(tip).toContain('Display');
  });

  it('shows traits when the backend sends them and works without them', () => {
    mount({ ...info(), traits: ['Stencil', 'Rounded'] } as ClassificationDto);
    expect(screen.getByTestId('class-chip').textContent).toContain('Stencil');
  });

  it('Uncertain shows a dimmed "có lẽ là {candidate}"', () => {
    mount(info({ class: 'uncertain', label: 'Uncertain', subclass: null, confidence: 0.5, candidate: 'script' }));
    const chip = screen.getByTestId('class-chip');
    expect(chip.textContent).toContain('Uncertain');
    expect(chip.textContent).toContain('có lẽ là Script');
    expect(chip.getAttribute('data-uncertain')).toBe('true');
  });

  it('override: "Đã sửa" with the auto class; menu sets and clears through the api', async () => {
    const { api, onChanged } = mount(info({ source: 'override', autoClass: 'sans', class: 'script', label: 'Script', subclass: null }));
    expect(screen.getByText('Đã sửa')).toBeTruthy();
    expect(await tipOf(screen.getByText('Đã sửa'))).toContain('Sans');
    const clear = vi.spyOn(api, 'clearClassificationOverride');
    fireEvent.click(screen.getByTestId('class-chip'));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Về tự động' }));
    await waitFor(() => expect(clear).toHaveBeenCalledWith(face));
    expect(onChanged).toHaveBeenCalled();
  });

  it('picking a class from the menu writes an override and Esc closes the menu', async () => {
    const { api, onChanged } = mount(info());
    const set = vi.spyOn(api, 'setClassificationOverride');
    fireEvent.click(screen.getByTestId('class-chip'));
    expect(screen.queryByRole('menuitem', { name: 'Về tự động' })).toBeNull();
    expect(screen.queryByRole('menuitemradio', { name: /Display/ })).toBeNull();
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    fireEvent.click(screen.getByTestId('class-chip'));
    fireEvent.click(screen.getByRole('menuitemradio', { name: /^Script/ }));
    await waitFor(() => expect(set).toHaveBeenCalledWith({ face, class: 'script', subclass: null }));
    expect(onChanged).toHaveBeenCalled();
  });

  it('opening the menu focuses the current class; arrows move; Esc closes and returns focus to the chip', () => {
    mount(info({ class: 'sans', label: 'Sans', subclass: null }));
    const chip = screen.getByTestId('class-chip');
    fireEvent.click(chip);
    const items = screen.getAllByRole('menuitemradio');
    expect(document.activeElement).toBe(screen.getByRole('menuitemradio', { name: /^Sans/ }));
    expect(screen.getByRole('menuitemradio', { name: /^Sans/ }).getAttribute('aria-checked')).toBe('true');
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(items[2]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' }); fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(items[0]);
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(chip);
  });

  it('Tab closes the menu and the chip is spoken with its value', () => {
    mount(info());
    const chip = screen.getByTestId('class-chip');
    expect(chip.getAttribute('aria-label')).toBe('Serif, Didone — Đổi phân loại');
    fireEvent.click(chip);
    fireEvent.keyDown(document.activeElement!, { key: 'Tab' });
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('a failing override shows an error notice, does not call onChanged and does not reload the grid', async () => {
    useNoticeStore.setState({ notices: [] });
    const api = createMockApi();
    const rev = useUiStore.getState().gridRev;
    vi.spyOn(api, 'setClassificationOverride').mockRejectedValue(new Error('db locked'));
    const { onChanged } = mount(info(), api);
    fireEvent.click(screen.getByTestId('class-chip'));
    fireEvent.click(screen.getByRole('menuitemradio', { name: /^Script/ }));
    await waitFor(() => expect(useNoticeStore.getState().notices[0]?.text).toContain('db locked'));
    expect(onChanged).not.toHaveBeenCalled();
    expect(useUiStore.getState().gridRev).toBe(rev);
  });

  it('a successful override reloads the grid (class filters must follow)', async () => {
    const rev = useUiStore.getState().gridRev;
    mount(info());
    fireEvent.click(screen.getByTestId('class-chip'));
    fireEvent.click(screen.getByRole('menuitemradio', { name: /^Script/ }));
    await waitFor(() => expect(useUiStore.getState().gridRev).toBe(rev + 1));
  });

  it('tooltip uses translated signal and usage labels', async () => {
    mount(info());
    const tip = await tipOf(screen.getByTestId('class-chip'));
    expect(tip).toContain('Số đo');
    expect(tip).toContain('Display (tiêu đề lớn)');
  });

  it('renders unknown signal votes, sources and traits instead of crashing', async () => {
    mount(info({ signals: [{ source: 'future_source', detail: 'x', vote: 'usage', weight: 0.1 }], traits: ['Quirky_new_trait'] }));
    expect(screen.getByTestId('class-chip').textContent).toContain('Quirky new trait');
    expect(await tipOf(screen.getByTestId('class-chip'))).toContain('future_source');
  });
});
