import { describe, expect, it } from 'vitest';
import type { CompareMetric } from '../../../api/compare';
import { deltaOf, deltaText, fitScale, percentLabel, rowUnit } from '../compareMath';

const m = (value: number | null, o: Partial<CompareMetric> = {}): CompareMetric => ({ key: 'k', value, unit: 'em', quality: 'measured', reason: null, percentile: 50, ...o });

describe('deltaOf', () => {
  it('is a percentage against the base for ratios and lengths', () => {
    expect(deltaOf('x_height_em', m(0.514), m(0.528))).toEqual({ kind: 'percent', value: expect.closeTo(-2.65, 1) });
    expect(deltaOf('contrast', m(4.89), m(1.31))).toEqual({ kind: 'percent', value: expect.closeTo(273.3, 0) });
  });
  it('is absolute degrees for slant (a base of 0 has no percentage)', () => {
    expect(deltaOf('slant_deg', m(-1.5, { unit: 'deg' }), m(0, { unit: 'deg' }))).toEqual({ kind: 'degrees', value: -1.5 });
  });
  it('is null when either side is not measured', () => {
    expect(deltaOf('x_height_em', m(null, { quality: 'failed' }), m(0.5))).toBeNull();
    expect(deltaOf('x_height_em', m(0.5), m(null, { quality: 'failed' }))).toBeNull();
  });
  it('is null against a zero or near-zero base for percentage metrics', () => {
    expect(deltaOf('contrast', m(2), m(0))).toBeNull();
    expect(deltaOf('contrast', m(2), m(0.0004))).toBeNull();
  });
});

describe('deltaOf with backend deltas', () => {
  it('uses the backend absolute delta for angles and the backend percent otherwise', () => {
    expect(deltaOf('slant_deg', m(5, { unit: 'deg', delta: { abs: 4.5, pct: 900 } }), m(0.5, { unit: 'deg' }))).toEqual({ kind: 'degrees', value: 4.5 });
    expect(deltaOf('contrast', m(2, { delta: { abs: 0.7, pct: 53.8 } }), m(1.3))).toEqual({ kind: 'percent', value: 53.8 });
  });
  it('a backend null delta means no delta (never recomputed)', () => {
    expect(deltaOf('contrast', m(2, { delta: { abs: null, pct: null } }), m(1.3))).toBeNull();
    expect(deltaOf('contrast', m(2, { delta: null }), m(1.3))).toBeNull();
  });
});

describe('deltaText', () => {
  it('uses Vietnamese decimals, the real minus sign and ±0 for tiny changes', () => {
    expect(deltaText({ kind: 'percent', value: -2.65 })).toEqual({ text: 'Δ −2,7%', tone: 'down' });
    expect(deltaText({ kind: 'percent', value: 273.3 })).toEqual({ text: 'Δ +273%', tone: 'up' });
    expect(deltaText({ kind: 'percent', value: 0.4 })).toEqual({ text: '±0', tone: 'zero' });
    expect(deltaText({ kind: 'degrees', value: -1.5 })).toEqual({ text: 'Δ −1,5°', tone: 'down' });
    expect(deltaText({ kind: 'degrees', value: 0.2 })).toEqual({ text: '±0', tone: 'zero' });
  });
});

describe('fitScale (Match x-height)', () => {
  it('scales a column so its x-height equals the base', () => {
    expect(fitScale(0.528, 0.464)).toBeCloseTo(1.1379, 3);
    expect(fitScale(0.528, 0.528)).toBe(1);
  });
  it('clamps extreme ratios to 0.5–2', () => {
    expect(fitScale(0.9, 0.2)).toBe(2);
    expect(fitScale(0.2, 0.9)).toBe(0.5);
  });
  it('does not scale when either x-height is unknown or absurd', () => {
    expect(fitScale(null, 0.5)).toBe(1);
    expect(fitScale(0.5, null)).toBe(1);
    expect(fitScale(0.5, 0)).toBe(1);
    expect(fitScale(0.5, 0.01)).toBe(1);
  });
});

describe('percentLabel / rowUnit', () => {
  it('formats percentile as P{n} clamped to 0–100 and null as empty', () => {
    expect(percentLabel(58.4)).toBe('P58');
    expect(percentLabel(120)).toBe('P100');
    expect(percentLabel(null)).toBe('');
  });
  it('picks the unit text per metric from the first measured column', () => {
    expect(rowUnit([m(null, { unit: null }), m(1, { unit: 'deg' })])).toBe('deg');
    expect(rowUnit([m(null, { unit: null })])).toBeNull();
  });
});
