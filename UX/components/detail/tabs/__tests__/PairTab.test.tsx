import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { FaceDetailDto } from '../../../../api';
import { useUiStore } from '../../../../store/ui';
import { PairTab } from '../PairTab';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
async function setup(pred: (r: { family: string; viMissing: number }) => boolean) {
  const api = createMockApi();
  const r = (await api.queryFaces({ limit: 200 })).items.find(pred)!;
  const detail: FaceDetailDto = await api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
  return { api, detail };
}

describe('PairTab', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('loads safe candidates for the current font as heading and shows "1 / N" with summary and relation labels', async () => {
    const { api, detail } = await setup(r => r.family === 'Playfair Display');
    const spy = vi.spyOn(api, 'pairCandidates');
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    expect(spy.mock.calls[0][1]).toMatchObject({ role: 'heading', mode: 'safe', projectVi: true });
    expect(screen.getByTestId('pair-counter').textContent).toMatch(/^1 \/ \d+$/);
    expect(screen.getByTestId('pair-summary').textContent).toMatch(/đi cùng Playfair Display/);
    const chips = screen.getAllByTestId('pair-label');
    expect(chips.length).toBeGreaterThanOrEqual(3);
    for (const c of chips) expect(c.textContent).toMatch(/^[●◐◆○◑] /);
    expect(chips.some(c => c.textContent!.includes('◆'))).toBe(false); // An toàn không có va chạm
  });

  it('◀ ▶ walk the candidates; the three modes refetch; experimental can show ◆', async () => {
    const { api, detail } = await setup(r => r.family === 'Playfair Display');
    const spy = vi.spyOn(api, 'pairCandidates');
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    const first = screen.getByTestId('pair-summary').textContent;
    fireEvent.click(screen.getByRole('button', { name: 'Ứng viên sau' }));
    expect(screen.getByTestId('pair-counter').textContent).toMatch(/^2 \//);
    expect(screen.getByTestId('pair-summary').textContent).not.toBe(first);
    fireEvent.click(screen.getByRole('button', { name: 'Ứng viên trước' }));
    expect(screen.getByTestId('pair-counter').textContent).toMatch(/^1 \//);
    expect(screen.getAllByRole('button', { pressed: true }).map(b => b.textContent)).toContain('An toàn');
    fireEvent.click(screen.getByRole('button', { name: 'Thử nghiệm' }));
    await settle();
    expect(spy.mock.calls.at(-1)![1].mode).toBe('experimental');
    expect(screen.getAllByTestId('pair-label').some(c => c.textContent!.includes('◆'))).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Có kiểm soát' }));
    await settle();
    expect(spy.mock.calls.at(-1)![1].mode).toBe('controlled');
  });

  it('role toggle sends role=body', async () => {
    const { api, detail } = await setup(r => r.family === 'Playfair Display');
    const spy = vi.spyOn(api, 'pairCandidates');
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Font này là nội dung' }));
    await settle();
    expect(spy.mock.calls.at(-1)![1].role).toBe('body');
  });

  it('a heading font missing Vietnamese, with "Dự án dùng tiếng Việt" on, says why and offers the missing characters', async () => {
    const { api, detail } = await setup(r => r.viMissing > 0);
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    expect(screen.getByText(/thiếu ký tự tiếng Việt/)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Xem ký tự thiếu' }));
    expect(useUiStore.getState().panelTab).toBe('vi');
  });

  it('turning off "Dự án dùng tiếng Việt" brings candidates back for that font', async () => {
    const { api, detail } = await setup(r => r.viMissing > 0);
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    expect(screen.queryByTestId('pair-counter')).toBeNull();
    fireEvent.click(screen.getByRole('switch', { name: 'Dự án dùng tiếng Việt' }));
    await settle();
    expect(screen.getByTestId('pair-counter')).toBeTruthy();
  });

  it('shows the mode-specific empty message when there is nothing', async () => {
    const { api, detail } = await setup(r => r.family === 'Playfair Display');
    vi.spyOn(api, 'pairCandidates').mockResolvedValue([]);
    render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    expect(screen.getByText(/Không có ứng viên ở chế độ An toàn/)).toBeTruthy();
    expect(screen.getByText(/Không có ứng viên/).textContent).toMatch(/Có kiểm soát/);
  });

  it('previews heading and body text, with no numeric score anywhere', async () => {
    const { api, detail } = await setup(r => r.family === 'Playfair Display');
    const { container } = render(<PairTab api={api} detail={detail} css={{}} />);
    await settle();
    expect(screen.getByTestId('pair-heading').textContent).toMatch(/\S/);
    expect(screen.getByTestId('pair-body').textContent).toMatch(/Hà Nội/);
    expect(container.textContent).not.toMatch(/%/);
  });
});
