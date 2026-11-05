import { describe, expect, it } from 'vitest';
import { createMockLibraryApi, makeFontUrl, makePreviewUrl, makeRow } from '../api';

const toUrl = (n: string) => `fontatlas-preview://localhost/${encodeURIComponent(n)}`;
const H = 'ab'.repeat(32);

describe('previewUrl', () => {
  it('builds file name, NFC text, generation and renderer version', () => {
    const url = makePreviewUrl(toUrl)({ hashHex: H, faceIndex: 1 }, { text: 'Ắn phở', px: 48, generation: 3, rendererVersion: 'preview-0.1.0' });
    expect(url).toBe(`fontatlas-preview://localhost/${H}_1_48.png?t=${encodeURIComponent('Ắn phở')}&g=3&r=preview-0.1.0`);
  });
  it('NFD and NFC text give the same url (no double fetch)', () => {
    const p = makePreviewUrl(toUrl);
    const o = { px: 48 as const, generation: 0, rendererVersion: 'v' };
    expect(p({ hashHex: H, faceIndex: 0 }, { ...o, text: 'Ắn' })).toBe(p({ hashHex: H, faceIndex: 0 }, { ...o, text: 'Ắn' }));
  });
});

describe('fontUrl', () => {
  it('only for face 0', () => {
    const f = makeFontUrl(n => `fontatlas-font://localhost/${n}`);
    expect(f({ hashHex: H, faceIndex: 0 })).toBe(`fontatlas-font://localhost/${H}_0.font`);
    expect(f({ hashHex: H, faceIndex: 1 })).toBeNull();
  });
});

describe('mock api', () => {
  const rows = Array.from({ length: 1500 }, (_, i) => makeRow(i));
  it('pages with total and clamps limit to 1000', async () => {
    const api = createMockLibraryApi(rows);
    const p = await api.queryFaces({ offset: 1400, limit: 200 });
    expect(p.total).toBe(1500);
    expect(p.items).toHaveLength(100);
    expect(p.items[0].faceId).toBe(rows[1400].faceId);
    expect((await api.queryFaces({ limit: 5000 })).items).toHaveLength(1000);
  });
  it('filters by folded text like the backend (đ → d, no diacritics)', async () => {
    const api = createMockLibraryApi([...rows.slice(0, 10), makeRow(999, { family: 'Đẹp Sans' })]);
    const p = await api.queryFaces({ text: 'dep' });
    expect(p.items.map(r => r.family)).toEqual(['Đẹp Sans']);
    expect(api.calls.at(-1)).toEqual({ text: 'dep' });
  });
  it('owns the generation like the backend', async () => {
    const api = createMockLibraryApi(rows);
    expect((await api.previewInfo()).generation).toBe(0);
    expect(await api.bumpGeneration()).toBe(1);
    expect((await api.previewInfo()).generation).toBe(1);
    expect(api.currentGeneration()).toBe(1);
  });
});
