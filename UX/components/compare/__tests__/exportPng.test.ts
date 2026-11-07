import { describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { exportFileName, clipGraphemes, ensureFonts, fitText, renderCompareBlob } from '../exportPng';

const ctx = { measureText: (s: string) => ({ width: s.length * 10 }) } as unknown as CanvasRenderingContext2D;

describe('fitText', () => {
  it('keeps text that fits and cuts the rest with an ellipsis within the width', () => {
    expect(fitText(ctx, 'abc', 100)).toBe('abc');
    const out = fitText(ctx, 'Playfair Display Black Italic', 100);
    expect(out.endsWith('…')).toBe(true);
    expect(ctx.measureText(out).width).toBeLessThanOrEqual(100);
  });
  it('can reduce to just the ellipsis', () => {
    expect(fitText(ctx, 'abcdef', 10)).toBe('…');
  });
});

describe('clipGraphemes', () => {
  it('never splits a base letter from its combining marks', () => {
    const s = 'ệ'.repeat(5); // ệ dạng tổ hợp ×5
    const out = clipGraphemes(s, 3);
    expect(out).toBe('ệ'.repeat(3));
  });
  it('leaves short text alone', () => {
    expect(clipGraphemes('Phở bò', 18)).toBe('Phở bò');
  });
});

describe('exportFileName', () => {
  it('is a safe .png name built from the families', async () => {
    const api = createMockApi();
    const d = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    const name = exportFileName(d);
    expect(name).toMatch(/^compare-[\p{L}\p{N}_-]+\.png$/u);
    expect(name.length).toBeLessThanOrEqual(120);
  });
});

describe('ensureFonts', () => {
  it('waits for every family to load (bounded) before drawing', async () => {
    const load = vi.fn().mockResolvedValue([]);
    Object.defineProperty(document, 'fonts', { value: { load }, configurable: true });
    await ensureFonts(['A', null, 'B', 'A'], 'Phở');
    expect(load).toHaveBeenCalledTimes(4); // 2 họ × (mẫu + dấu chồng)
    expect(load.mock.calls.every(c => /"(A|B)"/.test(c[0]))).toBe(true);
    Object.defineProperty(document, 'fonts', { value: undefined, configurable: true });
  });
  it('does nothing without the Font Loading API', async () => {
    await expect(ensureFonts(['A'], 'x')).resolves.toBeUndefined();
  });
});

describe('renderCompareBlob', () => {
  it('throws clearly when there is no 2D canvas (so the UI never reports a fake export)', async () => {
    const api = createMockApi();
    const data = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    await expect(renderCompareBlob({ data, families: new Map(), sample: 'x', fit: false, dark: false })).rejects.toThrow(/canvas/);
  });
});
