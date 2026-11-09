import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { inkProbe, measureInkWith, measureLoadedInk, resolveGuides, useInk } from '../inkLines';

describe('measureInkWith', () => {
  const mk = (h: number, x: number) => ({ font: '', measureText: (s: string) => ({ actualBoundingBoxAscent: s === 'H' ? h : x }) }) as unknown as CanvasRenderingContext2D;
  it('returns em ratios of H and x ascent', () => {
    expect(measureInkWith(mk(71, 52), '100px "A"')).toEqual({ cap: 0.71, x: 0.52 });
  });
  it('null when nothing measurable', () => {
    expect(measureInkWith(mk(0, 0), '100px "A"')).toBeNull();
  });
});

describe('resolveGuides', () => {
  it('ok within 1.5% em', () => {
    expect(resolveGuides({ cap: 0.7, x: 0.5 }, { cap: 0.71, x: 0.505 }).status).toBe('ok');
  });
  it('mismatch beyond 1.5% em keeps the glyph values', () => {
    const g = resolveGuides({ cap: 0.7, x: 0.5 }, { cap: 0.75, x: 0.5 });
    expect(g.status).toBe('mismatch');
    if (g.status === 'mismatch') { expect(g.cap).toBe(0.75); expect(g.analyzer.cap).toBe(0.7); }
  });
  it('unavailable without ink — never falls back to analyzer numbers', () => {
    expect(resolveGuides({ cap: 0.7, x: 0.5 }, null)).toEqual({ status: 'unavailable' });
  });
});


describe('measureLoadedInk', () => {
  it('a family that no FontFace matches (load() returns []) is NOT measured with the fallback font', async () => {
    const ctx = { font: '', measureText: () => ({ actualBoundingBoxAscent: 70 }) };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation((() => ctx) as never);
    const check = vi.fn(() => true); // check() nói true dù không có font
    const load = vi.fn().mockResolvedValueOnce([]).mockResolvedValueOnce([{}]);
    Object.defineProperty(document, 'fonts', { value: { load, check }, configurable: true });
    expect(await measureLoadedInk('Nope')).toBeNull();
    expect(await measureLoadedInk('Real')).toEqual({ cap: 0.7, x: 0.7 });
    Object.defineProperty(document, 'fonts', { value: undefined, configurable: true });
  });
});

describe('useInk', () => {
  it('after the family changes, the first render never reports the previous family ink', async () => {
    const seen: Array<unknown> = [];
    vi.spyOn(inkProbe, 'measure').mockImplementation(async f => ({ cap: f === 'A' ? 0.7 : 0.6, x: f === 'A' ? 0.5 : 0.4 }));
    const { result, rerender } = renderHook(({ f }) => { const v = useInk(f); seen.push(v); return v; }, { initialProps: { f: 'A' as string | null } });
    await waitFor(() => expect(result.current).toEqual({ cap: 0.7, x: 0.5 }));
    seen.length = 0;
    rerender({ f: 'B' });
    expect(seen[0]).toBeUndefined(); // không phải ink của A
    await waitFor(() => expect(result.current).toEqual({ cap: 0.6, x: 0.4 }));
    expect(seen.every(v => v === undefined || (v as { cap: number }).cap === 0.6)).toBe(true);
  });
  it('a family that does not exist resolves to null (unavailable), not a stale value', async () => {
    vi.spyOn(inkProbe, 'measure').mockResolvedValue(null);
    const { result } = renderHook(() => useInk('Ghost'));
    await waitFor(() => expect(result.current).toBeNull());
  });
});
