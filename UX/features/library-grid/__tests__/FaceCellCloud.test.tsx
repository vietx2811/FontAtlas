import { act, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { tipOf } from '../../../test/tip';
import { t } from '../../../i18n';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};

describe('FaceCell cloud placeholder', () => {
  it('only a card whose every copy is a cloud placeholder carries the note; the tooltip explains the auto-download', async () => {
    const rows = [makeRow(0, { onCloudOnly: true }), makeRow(1)];
    const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
    render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs()} fontSize={34} />);
    await settle();
    const [a, b] = screen.getAllByRole('gridcell');
    const note = within(a).getByTestId('cloud-only-note');
    expect(await tipOf(note)).toBe(t('cloud.note'));
    expect(t('cloud.note')).toMatch(/Trên cloud/);
    expect(t('cloud.note')).toMatch(/chưa tải về/);
    expect(within(b).queryByTestId('cloud-only-note')).toBeNull();
  });
});

// ---- xem trước của font cloud: đang tải / lỗi / công tắc tắt ----
const holdBlobs = (status?: number) => {
  const pending: Array<(s?: number) => void> = [];
  const d: BlobDeps = {
    fetch: vi.fn(() => new Promise(resolve => {
      const fire = (s = status ?? 200) => resolve({ ok: s === 200, status: s, blob: async () => new Blob(['p']), text: async () => ({ 504: 'cloud_timeout', 502: 'cloud_unavailable', 404: 'not_found', 403: 'cloud_disabled', 503: 'cloud_busy' } as Record<number, string>)[s] ?? '' });
      if (status !== undefined) fire(status); else pending.push(fire);
    })) as BlobDeps['fetch'],
    createObjectURL: () => 'blob:x', revokeObjectURL: () => {},
  };
  return { blobs: new PreviewBlobs(d), d, pending };
};
async function mountRows(rows: ReturnType<typeof makeRow>[], blobs: PreviewBlobs) {
  const loader = { acquire: vi.fn(async () => 'fa'), release: vi.fn(), onScroll: vi.fn() } as never;
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={loader} blobs={blobs} fontSize={34} />);
  await settle();
  return screen.getAllByRole('gridcell');
}

describe('FaceCell cloud preview states', () => {
  it('while the font is downloading the card shows a skeleton and "đang tải", not the plain placeholder note', async () => {
    const { blobs } = holdBlobs();
    const [a, b] = await mountRows([makeRow(0, { onCloudOnly: true }), makeRow(1)], blobs);
    const got = await tipOf(within(a).getByTestId('cloud-loading-note'));
    expect(got).toBe(t('cloud.loading'));
    expect(t('cloud.loading')).toMatch(/Trên cloud — đang tải/);
    expect(within(a).queryByTestId('cloud-only-note')).toBeNull();
    expect(a.querySelector('[data-cloud-skeleton]')).not.toBeNull();
    expect(within(b).queryByTestId('cloud-loading-note')).toBeNull();
    expect(b.querySelector('[data-cloud-skeleton]')).toBeNull();
  });

  it.each([
    [504, 'cloud.errTimeout'],
    [502, 'cloud.errFailed'],
    [403, 'cloud.errOff'],
    [404, 'cloud.errMissing'],
    [503, 'cloud.errBusy'],
  ] as const)('a %i answer shows the matching error note and the fallback text', async (status, key) => {
    const { blobs } = holdBlobs(status);
    const [a] = await mountRows([makeRow(0, { onCloudOnly: true })], blobs);
    expect(await tipOf(within(a).getByTestId('cloud-error-note'))).toBe(t(key));
    expect(within(a).getByText('Không có preview')).toBeTruthy();
    expect(within(a).queryByTestId('cloud-loading-note')).toBeNull();
  });

  it('an error on a local font keeps the old behaviour: no cloud note', async () => {
    const { blobs } = holdBlobs(404);
    const [a] = await mountRows([makeRow(0)], blobs);
    expect(within(a).queryByTestId('cloud-error-note')).toBeNull();
    expect(within(a).getByText('Không có preview')).toBeTruthy();
  });

  it('only 4 cloud previews are requested at once; local ones are not held back', async () => {
    const { blobs, d } = holdBlobs();
    await mountRows([...Array.from({ length: 6 }, (_, i) => makeRow(i, { onCloudOnly: true })), makeRow(6), makeRow(7)], blobs);
    expect(d.fetch).toHaveBeenCalledTimes(4 + 2);
  });
});
