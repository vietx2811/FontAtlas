import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { PairThesePage } from '../PairThesePage';

const initial = useUiStore.getState();
const rows = mockRows();
const faces = (n: number) => rows.slice(0, n).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
beforeEach(() => useUiStore.setState(initial, true));
const mount = async (api = createMockApi(), n = 3) => { render(<PairThesePage api={api} faces={faces(n)} />); await screen.findAllByTestId('pair-combo'); return api; };

describe('PairThesePage', () => {
  it('lists 1–3 ranked combos, each with a heading specimen, a body paragraph and both font names', async () => {
    await mount();
    const combos = screen.getAllByTestId('pair-combo');
    expect(combos.length).toBeGreaterThanOrEqual(1); expect(combos.length).toBeLessThanOrEqual(3);
    expect(within(combos[0]).getByText('Combo 1')).toBeTruthy();
    expect(within(combos[0]).getByTestId('pair-heading')).toBeTruthy();
    expect(within(combos[0]).getByTestId('pair-body')).toBeTruthy();
    expect(within(combos[0]).getByText('Tiêu đề')).toBeTruthy();
    expect(within(combos[0]).getByText('Nội dung')).toBeTruthy();
  });

  it('shows the ●◐◆○ labels with axis names, the summary and the gate', async () => {
    await mount();
    const c = screen.getAllByTestId('pair-combo')[0];
    expect(within(c).getAllByTestId('pair-label').length).toBeGreaterThan(0);
    expect(within(c).getByTestId('pair-summary').textContent).toBeTruthy();
    expect(within(c).getByTestId('pair-gate').textContent).toMatch(/An toàn|Có kiểm soát|Thử nghiệm/);
  });

  it('shows clashes, overflow and unsure data as warnings; unknown body warnings fall back to their key', async () => {
    const api = createMockApi();
    const base = await api.pairThese(faces(3));
    const first = base[0];
    vi.spyOn(api, 'pairThese').mockResolvedValue([{ ...first, assessment: { ...first.assessment, clashes: ['contrast'], headingOverflow: true, insufficientData: true, bodyWarnings: ['low_x_height', 'brand_new_warning'], weightHint: 'Tăng độ đậm nội dung một nấc' } }]);
    await mount(api);
    const c = screen.getByTestId('pair-combo');
    expect(within(c).getByText(/Va chạm: /)).toBeTruthy();
    expect(within(c).getByText('Tiêu đề dễ tràn dấu')).toBeTruthy();
    expect(within(c).getByText('Chưa đủ dữ liệu')).toBeTruthy();
    expect(within(c).getByText('Nội dung có x-height thấp')).toBeTruthy();
    expect(within(c).getByText('brand_new_warning')).toBeTruthy();
    expect(within(c).getByText('Tăng độ đậm nội dung một nấc')).toBeTruthy();
  });

  it('the Vietnamese switch re-asks with projectVi and the empty state offers to turn it off', async () => {
    const api = createMockApi();
    await api.setSettings({ projectViDefault: true });
    const spy = vi.spyOn(api, 'pairThese');
    render(<PairThesePage api={api} faces={faces(3)} />);
    await screen.findAllByTestId('pair-combo');
    expect(spy).toHaveBeenLastCalledWith(faces(3), expect.objectContaining({ projectVi: true }));
    spy.mockResolvedValueOnce([]);
    fireEvent.click(screen.getByRole('switch', { name: 'Dự án dùng tiếng Việt' }));
    await screen.findByText('Không có Combo phù hợp');
    expect(spy).toHaveBeenLastCalledWith(faces(3), expect.objectContaining({ projectVi: false }));
  });

  it('shows an error with retry', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'pairThese').mockRejectedValueOnce(new Error('boom'));
    render(<PairThesePage api={api} faces={faces(2)} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('boom');
    spy.mockRestore();
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    await screen.findAllByTestId('pair-combo');
  });

  it('Esc and the back button close it; Esc yields to dialogs; focus moves to the title and returns', async () => {
    const outside = document.createElement('button'); document.body.append(outside); outside.focus();
    useUiStore.getState().openPairThese(faces(3));
    const { unmount } = render(<PairThesePage api={createMockApi()} faces={faces(3)} />);
    await screen.findAllByTestId('pair-combo');
    expect(document.activeElement).toBe(screen.getByRole('heading', { name: /Pair these/ }));
    const dlg = document.createElement('div'); dlg.setAttribute('role', 'dialog'); document.body.append(dlg);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().pairThese).not.toBeNull(); dlg.remove();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().pairThese).toBeNull();
    useUiStore.getState().openPairThese(faces(3));
    fireEvent.click(screen.getByRole('button', { name: 'Quay lại thư viện' }));
    expect(useUiStore.getState().pairThese).toBeNull();
    unmount(); expect(document.activeElement).toBe(outside); outside.remove();
  });

  it('never shows a numeric score', async () => {
    await mount();
    expect(document.body.textContent).not.toMatch(/điểm|score/i);
  });

  it('starts from the "project uses Vietnamese" setting without asking twice', async () => {
    const api = createMockApi();
    await api.setSettings({ projectViDefault: false });
    const spy = vi.spyOn(api, 'pairThese');
    render(<PairThesePage api={api} faces={faces(3)} />);
    await screen.findAllByTestId('pair-combo');
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith(faces(3), expect.objectContaining({ projectVi: false }));
  });
});
