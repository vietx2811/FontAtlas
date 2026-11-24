import { act, cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { tipOf } from '../../../test/tip';
import { useUiStore } from '../../../store/ui';
import { setLocale } from '../../../i18n';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
const rows = [makeRow(0), makeRow(1), makeRow(2)];
async function mount() {
  const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs()} fontSize={34} isActive={() => true} activeScopeOf={() => 'session'} />);
  await settle();
  return screen.getAllByRole('gridcell');
}
const initial = useUiStore.getState();
beforeEach(() => { useUiStore.setState(initial, true); });
afterEach(() => { cleanup(); setLocale('vi'); });

describe('FaceCell — lý do bật ở màn «Đang bật»', () => {
  it('chỉ hiện ở view «active»: icon + tên ngắn; «+N» khi nhiều lý do; tooltip liệt kê đủ', async () => {
    useUiStore.setState({
      view: 'active',
      activeReasons: new Map([
        [rows[0].hashHex, [{ kind: 'manual' }]],
        [rows[1].hashHex, [{ kind: 'collection', collectionId: 3 }, { kind: 'case', caseId: 5 }]],
      ]),
      activeReasonNames: new Map([[rows[1].hashHex, [{ collectionName: 'Khách X', appNames: [] }, { collectionName: 'Quán Q', appNames: [] }]]]),
    } as never);
    const [a, b, c] = await mount();
    expect(within(a).getByTestId('cell-reason').textContent).toBe('Thủ công');
    expect(within(a).queryByTestId('cell-reason-more')).toBeNull();
    expect(within(b).getByTestId('cell-reason').textContent).toBe('Case Quán Q+1'); // Case xếp trước Dự án
    expect(within(b).getByTestId('cell-reason').querySelector('svg')).toBeTruthy();
    expect(within(c).queryByTestId('cell-reason')).toBeNull();
    const tip = await tipOf(within(b).getByTestId('cell-reason'));
    expect(tip).toMatch(/Bật cả Case «Quán Q»/); expect(tip).toMatch(/Từ «Khách X»/);
  });

  it('view khác (Tất cả…): không vẽ lý do', async () => {
    useUiStore.setState({ view: 'all', activeReasons: new Map([[rows[0].hashHex, [{ kind: 'manual' }]]]) } as never);
    const [a] = await mount();
    expect(within(a).queryByTestId('cell-reason')).toBeNull();
  });

  it('tiếng Anh: «Manual» / «Project …» / «Auto-on»', async () => {
    setLocale('en');
    useUiStore.setState({ view: 'active', activeReasons: new Map([[rows[0].hashHex, [{ kind: 'collection', collectionId: 3 }]], [rows[1].hashHex, [{ kind: 'auto', app: 'Photoshop' }]], [rows[2].hashHex, [{ kind: 'manual' }]]]), activeReasonNames: new Map([[rows[0].hashHex, [{ collectionName: 'Acme', appNames: [] }]]]) } as never);
    const [a, b, c] = await mount();
    expect(within(a).getByTestId('cell-reason').textContent).toBe('Project Acme');
    expect(within(b).getByTestId('cell-reason').textContent).toBe('Auto-on');
    expect(within(c).getByTestId('cell-reason').textContent).toBe('Manual');
  });
});
