import { act, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { useNameClashStore, vfDisplayName } from '../../../store/nameClash';
import { setLocale, t } from '../../../i18n';
import { tipOf } from '../../../test/tip';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
const grid = async () => {
  const rows = [makeRow(0, { family: 'Oswald', subfamily: 'Regular', isVariable: true }), makeRow(1, { family: 'Oswald', subfamily: 'Regular' }), makeRow(2)];
  const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs()} fontSize={34} />);
  await settle();
  const all = screen.getAllByRole('gridcell');
  return [1, 2, 3].map(id => all.find(c => c.getAttribute('data-face-id') === String(id))!); // theo faceId, không theo thứ tự sắp xếp
};

describe('G7 tên trùng VF ↔ static', () => {
  beforeEach(() => { setLocale('vi'); act(() => useNameClashStore.getState().set([])); });

  it('vfDisplayName chỉ thêm VF cho bản VF, không thêm hai lần', () => {
    expect(vfDisplayName('Oswald Regular', 'vf')).toBe('Oswald Regular VF');
    expect(vfDisplayName('Oswald Regular VF', 'vf')).toBe('Oswald Regular VF');
    expect(vfDisplayName('Oswald Regular', 'static')).toBe('Oswald Regular');
    expect(vfDisplayName('Oswald Regular', undefined)).toBe('Oswald Regular');
  });

  it('không trùng: không hậu tố, không dấu !', async () => {
    const cells = await grid();
    expect(within(cells[0]).getByTestId('face-name').textContent).toBe('Oswald Regular');
    expect(screen.queryByTestId('name-clash-note')).toBeNull();
  });

  it('bản VF: hậu tố VF + dấu !; bản static: dấu ! nhưng giữ tên; font khác không đổi', async () => {
    act(() => useNameClashStore.getState().set([{ vfFaceId: 1, staticFaceIds: [2], psNames: ['Oswald-Regular'], exact: true }]));
    const cells = await grid();
    expect(within(cells[0]).getByTestId('face-name').textContent).toBe('Oswald Regular VF');
    expect(await tipOf(within(cells[0]).getByTestId('name-clash-note'))).toBe(t('clash.vf.tip'));
    expect(within(cells[1]).getByTestId('face-name').textContent).toBe('Oswald Regular');
    expect(await tipOf(within(cells[1]).getByTestId('name-clash-note'))).toBe(t('clash.static.tip'));
    expect(within(cells[2]).queryByTestId('name-clash-note')).toBeNull();
  });

  it('suy ra từ named instance (exact=false) dùng lời giải thích "chưa chắc"; có đủ vi và en', async () => {
    act(() => useNameClashStore.getState().set([{ vfFaceId: 1, staticFaceIds: [2], psNames: ['Oswald-Regular_Light'], exact: false }]));
    const cells = await grid();
    expect(await tipOf(within(cells[0]).getByTestId('name-clash-note'))).toBe(t('clash.vf.tipGuess'));
    const vi = t('clash.vf.tipGuess');
    setLocale('en');
    expect(t('clash.vf.tipGuess')).not.toBe(vi);
    expect(t('clash.vf.tipGuess')).toMatch(/seems to share a PostScript name/);
    expect(t('clash.static.tip')).toMatch(/shares a PostScript name/);
  });
});
