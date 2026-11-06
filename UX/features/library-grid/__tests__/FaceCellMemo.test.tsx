import { act, render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import type { FontFaceLoader } from '../fontFaceLoader';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import * as types from '../types';

const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => new PreviewBlobs({ fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} } as BlobDeps);

describe('FaceCell is memoised', () => {
  it('changing the selection re-renders only the cards whose selected flag changed', async () => {
    const rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    const api = createMockLibraryApi(rows); const b = blobs();
    const loader = { acquire: vi.fn(async () => null), release: vi.fn(), onScroll: vi.fn() } as unknown as FontFaceLoader;
    const onPick = vi.fn(); const onContextMenu = vi.fn(); const onSelect = vi.fn(); const onToggleFavorite = vi.fn();
    const key = (i: number) => `${rows[i].hashHex}:0`;
    const ui = (keys: string[]) => <LibraryGrid api={api} query={{}} width={1300} height={800} loader={loader} blobs={b} fontSize={34}
      selectedKeys={new Set(keys)} onPick={onPick} onContextMenu={onContextMenu} onSelect={onSelect} onToggleFavorite={onToggleFavorite} isActive={() => false} isFavorite={() => false} />;
    const r = render(ui([]));
    await settle();
    const spy = vi.spyOn(types, 'previewBox'); // gọi 1 lần ở mỗi lần LibraryGrid render và 1 lần ở mỗi lần FaceCell render
    r.rerender(ui([key(1)]));
    await settle();
    const afterOne = spy.mock.calls.length;
    expect(afterOne).toBeGreaterThan(0);
    expect(afterOne).toBeLessThanOrEqual(2); // lưới + đúng 1 thẻ vừa được chọn (không phải cả 12 thẻ)
    spy.mockClear();
    r.rerender(ui([key(1), key(5)]));
    await settle();
    expect(spy.mock.calls.length).toBeLessThanOrEqual(2);
    spy.mockClear();
    r.rerender(ui([key(1), key(5)])); // không đổi gì
    await settle();
    expect(spy.mock.calls.length).toBe(1); // chỉ lưới; không thẻ nào vẽ lại
  });
});
