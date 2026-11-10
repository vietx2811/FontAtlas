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
