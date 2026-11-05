import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid, type LibraryGridProps } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { cellMinWidth } from '../types';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
async function mount(rows = [makeRow(0), makeRow(1, { viMissing: 12, supportsVi: false, subfamily: 'Bold' })], extra: Partial<LibraryGridProps> = {}) {
  const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
  const r = render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs()} fontSize={34} {...extra} />);
  await settle();
  return r;
}
const cells = () => screen.getAllByRole('gridcell');

describe('FaceCell skin (prototype)', () => {
  it('cell min width follows 140 + size × 2.6', () => {
    expect(cellMinWidth(34)).toBeCloseTo(228.4);
    expect(cellMinWidth(18)).toBeCloseTo(186.8);
    expect(cellMinWidth(72)).toBeCloseTo(327.2);
  });

  it('derives the column count from that width', async () => {
    await mount();
    const cols = Math.max(1, Math.floor((1300 - 28) / (cellMinWidth(34) + 8)));
    expect(screen.getByRole('grid').getAttribute('aria-colcount')).toBe(String(cols));
  });

  it('shows ● for complete Vietnamese and −N (with a warning) when characters are missing, whatever the sample', async () => {
    await mount(undefined, { sampleText: 'abc' });
    const [a, b] = cells();
    const ok = within(a).getByTestId('vi-mark');
    expect(ok.textContent).toBe('●');
    expect(ok.getAttribute('title')).toBe('Đủ dấu tiếng Việt');
    const miss = within(b).getByTestId('vi-mark');
    expect(miss.textContent).toBe('−12');
    expect(miss.getAttribute('title')).toBe('Thiếu 12 ký tự tiếng Việt');
    expect(miss.getAttribute('data-warn')).toBe('true');
  });

  it('names the face tiny: family + subfamily', async () => {
    await mount();
    expect(within(cells()[1]).getByTestId('face-name').textContent).toBe('Family 00001 Bold');
  });

  it('shows the activation dot only for active faces', async () => {
    await mount(undefined, { isActive: r => r.faceId === 1 });
    expect(within(cells()[0]).queryByTestId('active-dot')).not.toBeNull();
    expect(within(cells()[1]).queryByTestId('active-dot')).toBeNull();
  });

  it('star reflects favorite state, toggles without selecting', async () => {
    const onToggleFavorite = vi.fn(); const onSelect = vi.fn();
    await mount(undefined, { isFavorite: r => r.faceId === 2, onToggleFavorite, onSelect });
    const s0 = within(cells()[0]).getByRole('button', { name: 'Yêu thích' });
    const s1 = within(cells()[1]).getByRole('button', { name: 'Yêu thích' });
    expect(s0.getAttribute('aria-pressed')).toBe('false');
    expect(s1.getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(s0);
    expect(onToggleFavorite).toHaveBeenCalledWith(expect.objectContaining({ faceId: 1 }));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it('list layout is one font per row', async () => {
    await mount(undefined, { layout: 'list' });
    expect(screen.getByRole('grid').getAttribute('aria-colcount')).toBe('1');
    expect(screen.getByRole('grid').getAttribute('data-layout')).toBe('list');
    expect(cells()[0].getAttribute('data-layout')).toBe('list');
  });

  it('picks the preview bucket nearest to the slider and scales the image box, without regenerating inside a bucket', async () => {
    const api = createMockLibraryApi([makeRow(0)]);
    const bump = vi.spyOn(api, 'bumpGeneration');
    const { rerender } = render(<LibraryGrid api={api} query={{}} width={1300} height={800} blobs={blobs()} fontSize={34} />);
    await settle();
    const box = () => cells()[0].querySelector('[data-preview-box]') as HTMLElement;
    expect(box().style.width).toBe(`${13 * 34}px`);
    rerender(<LibraryGrid api={api} query={{}} width={1300} height={800} blobs={blobs()} fontSize={36} />);
    await settle();
    expect(box().style.width).toBe(`${13 * 36}px`);
    expect(bump).not.toHaveBeenCalled(); // 34 và 36 cùng bucket 64
    rerender(<LibraryGrid api={api} query={{}} width={1300} height={800} blobs={blobs()} fontSize={60} />);
    await settle();
    expect(bump).toHaveBeenCalledTimes(1); // sang bucket 96
  });
});
