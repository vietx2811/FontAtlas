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

describe('renderCompare (PNG content)', () => {
  const fakeCtx = () => {
    const state = { fills: [] as string[] };
    const ctx = new Proxy({ font: '', fillStyle: '', measureText: (s: string) => ({ width: s.length * 7 }), getImageData: () => ({ data: [0, 0, 0, 255] }) } as Record<string, unknown>, {
      get: (o, k: string) => (k in o ? o[k] : () => undefined),
      set: (o, k: string, v) => { o[k] = v; if (k === 'fillStyle') state.fills.push(String(v)); return true; },
    });
    return { ctx, state };
  };
  const setup = async (dark: boolean, tweak?: (d: Awaited<ReturnType<ReturnType<typeof createMockApi>['compareFaces']>>) => void, overlay?: boolean) => {
    const { ctx, state } = fakeCtx();
    const spy = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation((() => ctx) as never);
    const api = createMockApi();
    const data = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    tweak?.(data);
    const { renderCompare } = await import('../exportPng');
    const r = renderCompare({ data, families: new Map(), sample: 'Phở', fit: false, dark, overlay });
    spy.mockRestore();
    return { r, state };
  };

  it('has the overlay row, units on every measure, and never an ellipsis', async () => {
    const { r } = await setup(false);
    expect(r!.manifest.rows).toEqual(expect.arrayContaining(['header', 'specimen', 'overlay', 'vi', 'class']));
    expect(r!.manifest.texts.some(x => /\d,\d{3} em$/.test(x) || /\d,\d ° ?$/.test(x) || /tỷ lệ/.test(x))).toBe(true);
    expect(r!.manifest.texts.some(x => x.includes('…'))).toBe(false);
  });
  it('wraps a long label instead of cutting it', async () => {
    const { r } = await setup(false);
    const joined = r!.manifest.texts.join(' ');
    expect(joined).toContain('vuông');
    expect(joined).toContain('góc)');
  });
  it('keeps the missing-characters note and the "marks fit" note', async () => {
    const { r } = await setup(false, d => { d.faces[0].vi.supports = false; d.faces[0].vi.missing = 68; d.faces[1].vi.supports = true; d.faces[1].vi.stackedMarksOverflow = false; });
    expect(r!.manifest.texts).toContain('Thiếu 68 ký tự');
    expect(r!.manifest.texts).toContain('Dấu vừa khung');
  });
  it('uses a dark specimen background in dark theme and white in light', async () => {
    const dark = await setup(true); const light = await setup(false);
    expect(dark.r!.manifest.specimenBg.toLowerCase()).toBe('#0a0908');
    expect(dark.state.fills).toContain('#0A0908');
    expect(light.r!.manifest.specimenBg.toLowerCase()).toBe('#ffffff');
  });
  it('can omit the overlay row when it is hidden on screen', async () => {
    const { r } = await setup(false, undefined, false);
    expect(r!.manifest.rows).not.toContain('overlay');
  });
});

describe('wrapText', () => {
  const m = { measureText: (s: string) => ({ width: s.length * 10 }) } as unknown as CanvasRenderingContext2D;
  it('wraps by word and hard-breaks an over-wide word, no ellipsis', async () => {
    const { wrapText } = await import('../exportPng');
    expect(wrapText(m, 'tương phản nét vuông góc', 120)).toEqual(['tương phản', 'nét vuông', 'góc']);
    expect(wrapText(m, 'abcdefghij', 50)).toEqual(['abcde', 'fghij']);
  });
});
