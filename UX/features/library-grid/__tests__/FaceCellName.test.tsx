import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { tipOf } from '../../../test/tip';
import { createMockLibraryApi, makeRow } from '../api';
import type { FontFaceLoader } from '../fontFaceLoader';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
/** Loader giả: mỗi lần acquire trả một promise ta tự giải, để thấy trạng thái "chưa nạp" rồi "đã nạp". */
function deferredLoader() {
  const waiting: Array<{ hash: string; index: number; resolve(v: string | null): void }> = [];
  const loader = {
    acquire: vi.fn((r: { hashHex: string; faceIndex: number }) => new Promise<string | null>(resolve => waiting.push({ hash: r.hashHex, index: r.faceIndex, resolve }))),
    release: vi.fn(), onScroll: vi.fn(),
  } as unknown as FontFaceLoader & { acquire: ReturnType<typeof vi.fn>; release: ReturnType<typeof vi.fn> };
  return { loader, waiting };
}
async function mount(rows = [makeRow(0, { family: 'Playfair Display', subfamily: 'Bold' }), makeRow(1, { family: 'Roboto', faceIndex: 1 })]) {
  const dl = deferredLoader();
  render(<LibraryGrid api={createMockLibraryApi(rows)} query={{}} width={1300} height={800} loader={dl.loader} blobs={blobs()} fontSize={34} />);
  await settle();
  return dl;
}
const names = () => screen.getAllByTestId('face-name');

describe('card title in its own font', () => {
  it('uses the UI font until the face is loaded, then the face family with the UI font as fallback', async () => {
    const { waiting } = await mount();
    expect(names()[0].style.fontFamily).toBe('');
    expect(names()[0]).not.toHaveAttribute('data-own-font');
    await act(async () => { waiting.find(w => w.index === 0)!.resolve(`fa-${'0'.repeat(64)}-0`); });
    const fam = names()[0].style.fontFamily;
    expect(fam).toContain(`fa-${'0'.repeat(64)}-0`);
    expect(fam).toContain('var(--font-ui)'); // glyph thiếu (font ký hiệu/không có Latin) rơi về font giao diện
    expect(names()[0].textContent).toBe('Playfair Display Bold');
    expect(names()[0].className).toContain('fa-own-font'); // weight/style về gốc, không in đậm giả
  });

  it('keeps the UI font for TTC faces ≥ 1 and when the loader gives nothing (fast scroll, load error)', async () => {
    const { waiting } = await mount();
    await act(async () => { waiting.forEach(w => w.resolve(null)); });
    names().forEach(n => expect(n.style.fontFamily).toBe(''));
  });

  it('acquires on show and releases on unmount (only visible cards hold fonts)', async () => {
    const dl = deferredLoader();
    const { unmount } = render(<LibraryGrid api={createMockLibraryApi([makeRow(0)])} query={{}} width={1300} height={800} loader={dl.loader} blobs={blobs()} fontSize={34} />);
    await settle();
    expect(dl.loader.acquire).toHaveBeenCalledWith(expect.objectContaining({ hashHex: '0'.repeat(64), faceIndex: 0 }), { soft: true }); // tên = ref mềm
    expect(dl.loader.release).not.toHaveBeenCalled();
    unmount();
    expect(dl.loader.release).toHaveBeenCalledWith(expect.objectContaining({ hashHex: '0'.repeat(64) }), { soft: true });
  });

  it('a name that is cut gets a tooltip with the full name; a name that fits does not', async () => {
    await mount([makeRow(0, { family: 'Một cái tên rất rất dài của một gia đình font', subfamily: 'Black Condensed' })]);
    const n = names()[0];
    expect(n).not.toHaveAttribute('title');
    expect(await tipOf(n)).toBeNull();
    Object.defineProperty(n, 'scrollWidth', { configurable: true, value: 300 });
    Object.defineProperty(n, 'clientWidth', { configurable: true, value: 120 });
    expect(await tipOf(n)).toBe('Một cái tên rất rất dài của một gia đình font Black Condensed');
  });

  it('badges, star and active dot all carry tooltips instead of native titles', async () => {
    await mount([makeRow(0, { viMissing: 3, supportsVi: false, path: '/f/a.otf' })]);
    expect(await tipOf(screen.getByTestId('face-format'))).toBe('Định dạng file OTF');
    expect(await tipOf(screen.getByTestId('vi-mark'))).toBe('Thiếu 3 ký tự tiếng Việt');
    expect(await tipOf(screen.getByRole('button', { name: 'Yêu thích' }))).toBe('Yêu thích');
    expect(document.querySelectorAll('[title]')).toHaveLength(0);
  });
});
