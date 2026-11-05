import { act, render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';
import { PX_BUCKETS, bucketFor, previewBox } from '../types';

/** PNG của Rust: rộng 13·px, cao ⌊7·px/3⌋ (canvas 2×). Tỉ lệ hiển thị phải đúng tỉ lệ này, sai quá 1% là chữ bị bóp/giãn. */
const pngRatio = (px: number) => (13 * px) / Math.floor((7 * px) / 3);
const ratioOk = (w: number, h: number, px: number) => Math.abs(w / h / pngRatio(px) - 1) <= 0.01;

describe('previewBox keeps the PNG aspect ratio', () => {
  it('for every bucket and every slider size 18..72 (±1%)', () => {
    for (const px of PX_BUCKETS) for (let s = 18; s <= 72; s++) {
      const b = previewBox(px, s);
      expect(ratioOk(b.width, b.height, px), `px=${px} size=${s} → ${b.width}×${b.height}`).toBe(true);
    }
  });
  it('and the bucket nearest to the slider is what the ratio uses', () => {
    for (let s = 18; s <= 72; s++) { const px = bucketFor(s); expect(ratioOk(previewBox(px, s).width, previewBox(px, s).height, px)).toBe(true); }
  });
});

const blobs = () => {
  const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:x', revokeObjectURL: () => {} };
  return new PreviewBlobs(d);
};
const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('the drawn preview is never squeezed by the card', () => {
  it('mask box has the exact image ratio, inside a wrapper that clips (overflow) instead of shrinking', async () => {
    render(<LibraryGrid api={createMockLibraryApi([makeRow(0)])} query={{}} width={1000} height={600} blobs={blobs()} fontSize={34} />);
    await settle();
    const wrap = screen.getAllByRole('gridcell')[0].querySelector('[data-preview-box]') as HTMLElement;
    const mask = wrap.querySelector('[data-mask]') as HTMLElement;
    const w = parseFloat(mask.style.width); const h = parseFloat(mask.style.height);
    expect(ratioOk(w, h, bucketFor(34))).toBe(true);
    expect(mask.parentElement).toBe(wrap);
    // Khung ngoài cắt phần tràn; chỉ khung ngoài có thể hẹp hơn ảnh.
    expect(wrap.style.width).toBe('');
    expect(wrap.style.minHeight).toBe(`${h}px`); // chiều cao ảnh là sàn; phần dư của ô dùng để căn giữa chữ mẫu
  });

  it('img fallback keeps width and height attributes in the same ratio', async () => {
    render(<LibraryGrid api={createMockLibraryApi([makeRow(0)])} query={{}} width={1000} height={600} blobs={blobs()} fontSize={50} previewMode="img" />);
    await settle();
    const img = screen.getAllByRole('gridcell')[0].querySelector('img') as HTMLImageElement;
    expect(ratioOk(Number(img.getAttribute('width')), Number(img.getAttribute('height')), bucketFor(50))).toBe(true);
  });
});

describe('CSS rules that used to squeeze the image', () => {
  const css = readFileSync(resolve(__dirname, '../libraryGrid.css'), 'utf8');
  const rule = (sel: string) => new RegExp(`(?:^|\\n)${sel.replace(/[.[\]]/g, '\\$&')}\\s*{([^}]*)}`).exec(css)?.[1] ?? '';
  it('the wrapper clips and has no max-width that could shrink only one axis', () => {
    expect(rule('.fa-spec')).toMatch(/overflow:\s*hidden/);
    expect(rule('.fa-spec')).toMatch(/align-items:\s*center/);
    expect(rule('.fa-spec')).not.toMatch(/max-width/);
  });
  it('the mask/img never shrink: flex none, max-width none', () => {
    for (const sel of ['.fa-preview-mask', '.fa-preview-img']) {
      expect(rule(sel), sel).toMatch(/flex:\s*none/);
      expect(rule(sel), sel).toMatch(/max-width:\s*none/);
    }
  });
});
