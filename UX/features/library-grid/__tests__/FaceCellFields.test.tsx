import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useCardFields } from '../../../store/cardFields';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => new PreviewBlobs({ fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} } as BlobDeps);
async function mount() {
  const rows = [makeRow(0, { family: 'Inter', isVariable: true, licenseClass: 'ofl' }), makeRow(1, { family: 'Lora', isVariable: false, licenseClass: 'unknown' })];
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={null} blobs={blobs()} fontSize={34} />);
  await settle();
}

describe('card fields on the font card', () => {
  beforeEach(() => { act(() => useCardFields.getState().reset()); });
  it('default: format and language badge only, no second row', async () => {
    await mount();
    expect(screen.queryAllByTestId('face-meta2')).toHaveLength(0);
    expect(screen.queryAllByTestId('face-license')).toHaveLength(0);
    expect(screen.getAllByTestId('vi-mark').length).toBeGreaterThan(0);
  });
  it('license and Variable turn on a second row; VAR only on variable faces; unchecking format hides it', async () => {
    await mount();
    act(() => useCardFields.getState().set(['format', 'lang', 'license', 'variable'])); await settle();
    expect(screen.getAllByTestId('face-meta2').length).toBe(2); // hàng 2 có ở mọi thẻ: chiều cao không nhảy
    expect(screen.getAllByTestId('face-license').map(e => e.textContent)).toEqual(['OFL', 'Chưa rõ']);
    expect(screen.getAllByTestId('face-variable')).toHaveLength(1);
    act(() => useCardFields.getState().set(['license'])); await settle();
    expect(screen.queryAllByTestId('face-format')).toHaveLength(0);
    expect(screen.queryAllByTestId('vi-mark')).toHaveLength(0); // bỏ chọn Ngôn ngữ thì mất huy hiệu
  });
});
