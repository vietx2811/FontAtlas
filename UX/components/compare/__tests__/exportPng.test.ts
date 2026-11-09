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
    expect(load).toHaveBeenCalledTimes(6); // 2 họ × (mẫu + dấu chồng + overlay 64px)
    expect(load.mock.calls.every(c => /"(A|B)"/.test(c[0]))).toBe(true);
    Object.defineProperty(document, 'fonts', { value: undefined, configurable: true });
  });
  it('does nothing without the Font Loading API', async () => {
    expect([...await ensureFonts(['A'], 'x')]).toEqual([]);
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
  const setup = async (dark: boolean, tweak?: (d: Awaited<ReturnType<ReturnType<typeof createMockApi>['compareFaces']>>) => void, overlay?: boolean | number, extra: { families?: boolean; missing?: Record<string, string> } = {}) => {
    const { ctx, state } = fakeCtx();
    const spy = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation((() => ctx) as never);
    const api = createMockApi();
    const data = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    tweak?.(data);
    const { renderCompare } = await import('../exportPng');
    const keys = data.faces.map(f => `${f.face.hashHex}:${f.face.faceIndex}`);
    const ov = overlay === false ? null : typeof overlay === 'number' ? keys.slice(0, overlay) : keys;
    const families = new Map(extra.families ? keys.map((k, i) => [k, `Fam${i}`] as const) : []);
    const missing = new Map(Object.entries(extra.missing ?? {}).map(([f, chars]) => [f, new Set([...chars])] as const));
    const r = renderCompare({ data, families, sample: 'Phở', fit: false, dark, overlay: ov, missing });
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
  it('can omit the overlay row when Overlay is off on screen, or when no font is selected', async () => {
    const { r } = await setup(false, undefined, false);
    expect(r!.manifest.rows).not.toContain('overlay');
    expect(r!.manifest.overlayLayers).toBe(0);
    const none = await setup(false, undefined, 0);
    expect(none.r!.manifest.rows).not.toContain('overlay');
  });
  it('draws one Overlay block with exactly the toggled fonts and a legend naming them', async () => {
    const two = await setup(false, undefined, true);
    expect(two.r!.manifest.overlayLayers).toBe(2);
    expect(two.r!.manifest.rows.filter(x => x === 'overlay')).toHaveLength(1);
    expect(two.r!.manifest.texts.filter(x => x.includes('· mốc'))).toHaveLength(1);
    const one = await setup(false, undefined, 1);
    expect(one.r!.manifest.overlayLayers).toBe(1);
  });
  it('missing characters are painted grey and listed in plain text (a PNG has no hover)', async () => {
    const { r, state } = await setup(false, undefined, false, { families: true, missing: { Fam0: 'ởố' } });
    expect(r!.manifest.texts.some(x => x.startsWith('Thiếu: '))).toBe(true);
    expect(state.fills).toContain('#78716C'); // --faint (sáng)
    const clean = await setup(false, undefined, false, { families: true });
    expect(clean.r!.manifest.texts.some(x => x.startsWith('Thiếu: '))).toBe(false);
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

describe('renderCompare: specimen text never overflows the column', () => {
  it('shrinks the whole row by one factor when the text is wider than the column, and leaves it at 1 otherwise', async () => {
    const mk = (perChar: number) => new Proxy({ font: '', fillStyle: '', measureText: (s: string) => ({ width: s.length * perChar, actualBoundingBoxAscent: 50 }) } as Record<string, unknown>, {
      get: (o, k: string) => (k in o ? o[k] : () => undefined), set: (o, k: string, v) => { o[k] = v; return true; },
    });
    const api = createMockApi();
    const data = await api.compareFaces(mockRows().slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    const { renderCompare } = await import('../exportPng');
    const run = (perChar: number) => {
      const c = mk(perChar);
      const spy = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation((() => c) as never);
      const r = renderCompare({ data, families: new Map(), sample: 'Phở bò Hà Nội', fit: false, dark: false });
      spy.mockRestore();
      return r!.manifest.fit;
    };
    expect(run(5)).toEqual({ specimen: 1, vi: 1 });
    const big = run(30);
    expect(big.specimen).toBeLessThan(1);
    expect(big.vi).toBeLessThan(1);
    expect(big.specimen * 13 * 30).toBeLessThanOrEqual(234 + 1e-6); // chữ sau khi thu vừa bề rộng khả dụng (CW-36)
  });
});


describe('ensureFonts resilience', () => {
  it('one rejecting font does not fail the export: others still load, the bad one is just not "loaded"', async () => {
    const load = vi.fn((font: string) => (font.includes('"Bad"') ? Promise.reject(new Error('boom')) : Promise.resolve([{}])));
    Object.defineProperty(document, 'fonts', { value: { load }, configurable: true });
    const clear = vi.spyOn(globalThis, 'clearTimeout');
    const out = await ensureFonts(['Good', 'Bad'], 'Phở');
    expect([...out]).toEqual(['Good']);
    expect(clear).toHaveBeenCalled(); // bộ đếm 4 giây được dọn
    expect(load.mock.calls.some(c => c[0] === '64px "Good"')).toBe(true); // overlay 64px được nạp
    clear.mockRestore();
    Object.defineProperty(document, 'fonts', { value: undefined, configurable: true });
  });
});

describe('review 6 · PNG follows the screen layout', () => {
  it('uses the pinned column order, skips hidden rows and collapsed groups', async () => {
    const { ctx } = (() => {
      const ctx = new Proxy({ measureText: (s: string) => ({ width: s.length * 7, actualBoundingBoxAscent: 0 }) } as Record<string, unknown>, { get: (o, k: string) => (k in o ? o[k] : () => undefined), set: (o, k: string, v) => { o[k] = v; return true; } });
      return { ctx };
    })();
    const spy = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation((() => ctx) as never);
    const api = createMockApi();
    const data = await api.compareFaces(mockRows().slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
    const keys = data.faces.map(f => `${f.face.hashHex}:${f.face.faceIndex}`);
    const { renderCompare } = await import('../exportPng');
    const full = renderCompare({ data, families: new Map(), sample: 'Phở', fit: false, dark: false, overlay: null })!;
    const piped = renderCompare({ data, families: new Map(), sample: 'Phở', fit: false, dark: false, overlay: null, order: [keys[2], keys[0], keys[1]], hiddenRows: new Set(['metrics.x_height_em']), collapsed: new Set(['class']) })!;
    spy.mockRestore();
    const baseName = `(mốc) ${data.faces[0].row.family}`;
    expect(full.manifest.rows).toContain('metric:x_height_em');
    expect(piped.manifest.rows).not.toContain('metric:x_height_em');
    expect(piped.manifest.rows).not.toContain('class');
    expect(piped.manifest.size.h).toBeLessThan(full.manifest.size.h);
    expect(piped.manifest.texts.some(x => x.startsWith('(mốc)'))).toBe(true);
    const names = piped.manifest.texts.filter(x => x.includes(data.faces[2].row.family) || x.includes(baseName.slice(6)));
    expect(names.length).toBeGreaterThan(0);
    const iThird = piped.manifest.texts.findIndex(x => x.includes(data.faces[2].row.family) && !x.startsWith('('));
    const iBase = piped.manifest.texts.findIndex(x => x.startsWith('(mốc)'));
    expect(iThird).toBeLessThan(iBase); // cột pin vẽ trước cột mốc
  });
});
