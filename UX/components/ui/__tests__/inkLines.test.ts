import { describe, expect, it } from 'vitest';
import { measureInkWith, resolveGuides } from '../inkLines';

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
