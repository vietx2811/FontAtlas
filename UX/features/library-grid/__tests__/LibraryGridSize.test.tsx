import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import type { FontFaceLoader } from '../fontFaceLoader';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { cellMinWidth } from '../types';

const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => new PreviewBlobs({ fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} } as BlobDeps);
const loader = { acquire: vi.fn(async () => null), release: vi.fn(), onScroll: vi.fn() } as unknown as FontFaceLoader;
const rows = Array.from({ length: 30 }, (_, i) => makeRow(i));
const FONT = 34;
const colsFor = (w: number) => Math.max(1, Math.floor((w - 28) / (cellMinWidth(FONT) + 8)));

/** Khung chứa có bề rộng do test điều khiển (jsdom không có layout). ResizeObserver của setup là no-op → mô phỏng WebView không chạy RO. */
let box = { w: 1500, h: 800 };
const spyW = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => box.w);
const spyH = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(() => box.h);

const ui = (selected: string[] = []) => (
  <LibraryGrid api={createMockLibraryApi(rows)} query={{}} loader={loader} blobs={blobs()} fontSize={FONT}
    selectedKeys={new Set(selected)} onSelect={() => {}} />
);
const cols = () => Number(screen.getByRole('grid').getAttribute('aria-colcount'));

describe('LibraryGrid follows its container without ResizeObserver', () => {
  afterEach(() => { box = { w: 1500, h: 800 }; });

  it('column count tracks the real width on the next render (panel opens → narrower), even if RO never fires', async () => {
    const r = render(ui());
    await settle();
    expect(cols()).toBe(colsFor(1500));
    expect(cols()).toBeGreaterThanOrEqual(4);
    box = { w: 1500 - 440, h: 800 };           // panel chi tiết mở: khung chứa hẹp lại 440 px
    r.rerender(ui(['a:0']));                    // Library vẽ lại vì có thẻ được chọn
    await settle();
    expect(cols()).toBe(colsFor(1500 - 440));
    expect(cols()).toBeLessThan(colsFor(1500));
  });

  it('…and wider again when the panel closes', async () => {
    box = { w: 1060, h: 800 };
    const r = render(ui());
    await settle();
    const narrow = cols();
    box = { w: 1500, h: 800 };
    r.rerender(ui());
    await settle();
    expect(cols()).toBeGreaterThan(narrow);
  });

  it('the virtual list always fills its container by CSS (never a pixel width from an older measurement)', async () => {
    const r = render(ui());
    await settle();
    const scroller = screen.getByTestId('library-grid-scroller');
    expect(scroller.style.width).toBe('100%');
    box = { w: 900, h: 800 };
    r.rerender(ui(['a:0']));
    await settle();
    expect(screen.getByTestId('library-grid-scroller').style.width).toBe('100%');
  });

  it('a container with no layout (0×0: hidden, jsdom) keeps the last known size instead of collapsing to 1 column', async () => {
    const r = render(ui());
    await settle();
    const before = cols();
    box = { w: 0, h: 0 };
    r.rerender(ui(['a:0']));
    await settle();
    expect(cols()).toBe(before);
  });
});

void spyW; void spyH;
