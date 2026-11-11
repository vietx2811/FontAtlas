import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { tipOf } from '../../../test/tip';
import { useUiStore } from '../../../store/ui';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
async function mount(props: Partial<React.ComponentProps<typeof LibraryGrid>>) {
  const rows = [makeRow(0), makeRow(1)];
  const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs()} fontSize={34} {...props} />);
  await settle();
  return screen.getAllByRole('gridcell');
}

describe('FaceCell activation', () => {
  it('the status dot is the toggle button: toggles Active for that row only and does not select the card', async () => {
    const onToggleActive = vi.fn(); const onSelect = vi.fn();
    const [a, b] = await mount({ onToggleActive, onSelect, isActive: r => r.faceId === 1 });
    const btn = within(b).getByTestId('cell-active');
    expect(btn.getAttribute('aria-label')).toBe('Bật/Tắt font');
    expect(btn.closest('.fa-meta')?.firstElementChild).toBe(btn); // đứng ngay đầu tên
    expect(btn.getAttribute('aria-pressed')).toBe('false');
    fireEvent.click(btn);
    expect(onToggleActive).toHaveBeenCalledTimes(1);
    expect(onToggleActive.mock.calls[0][0].faceId).toBe(2);
    expect(onSelect).not.toHaveBeenCalled();
    expect(within(a).getByTestId('cell-active')).toBeTruthy();
  });

  it('reflects the active state on the dot button; the tooltip names the scope', async () => {
    const [a, b] = await mount({ onToggleActive: vi.fn(), isActive: r => r.faceId === 1, activeScopeOf: () => 'persistent' });
    expect(within(a).getByTestId('cell-active').getAttribute('aria-pressed')).toBe('true');
    expect(within(b).queryByTestId('active-dot')).toBeNull();
    const dot = within(a).getByTestId('active-dot');
    expect(dot.getAttribute('data-scope')).toBe('persistent');
    expect(await tipOf(within(a).getByTestId('cell-active'))).toBe('Bật/Tắt font · Luôn bật');
    expect(await tipOf(within(b).getByTestId('cell-active'))).toBe('Bật/Tắt font · Chưa bật');
    expect(within(b).getByTestId('cell-active').querySelector('.fa-dot')?.getAttribute('data-scope')).toBe('off');
  });

  it('session scope dot says "Đang bật · phiên này"; no power button without a handler', async () => {
    const [a] = await mount({ isActive: () => true });
    expect(await tipOf(within(a).getByTestId('active-dot'))).toBe('Đang bật · phiên này');
    expect(within(a).queryByTestId('cell-active')).toBeNull();
  });

  it('an available-elsewhere font gets a hollow green dot with a tooltip saying who, and no power button', async () => {
    useUiStore.setState({ available: new Map([[makeRow(0).hashHex, { owner: 'font_base' as const, path: '/Users/u/Library/Fonts/FontBase/x.ttf' }]]) });
    const [a, b] = await mount({ onToggleActive: vi.fn(), activeScopeOf: r => (r.faceId === 1 ? 'available' : undefined) });
    const dot = within(a).getByTestId('active-dot');
    expect(dot.getAttribute('data-scope')).toBe('available');
    expect(await tipOf(dot)).toMatch(/^Có sẵn trên máy/);
    expect(within(a).queryByTestId('cell-active')).toBeNull();
    expect(within(b).queryByTestId('active-dot')).toBeNull();
    expect(within(b).getByTestId('cell-active')).toBeTruthy();
  });

  it('Space on the focused dot button toggles it without selecting the card (native button)', async () => {
    const onToggleActive = vi.fn(); const onSelect = vi.fn();
    const [a] = await mount({ onToggleActive, onSelect });
    const btn = within(a).getByTestId('cell-active');
    expect(btn.tagName).toBe('BUTTON');
    btn.focus(); fireEvent.keyDown(btn, { key: ' ' }); fireEvent.click(btn); // jsdom không tự đổi keyup→click
    expect(onToggleActive).toHaveBeenCalledTimes(1);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
