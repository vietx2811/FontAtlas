import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { chooseLocale, formatDate, formatNumber, getLocale, hasKey, resolveLocale, setLocale, t, vi, useLocale, type I18nKey } from '..';
import { en } from '../en';
import { LocaleBoundary } from '../LocaleBoundary';
import { localeFromSystem } from '../locale';

describe('locale tables', () => {
  it('every vi key has a non-empty en value, and the other way round', () => {
    const viKeys = Object.keys(vi).sort(); const enKeys = Object.keys(en).sort();
    expect(enKeys).toEqual(viKeys);
    for (const k of enKeys) expect((en as Record<string, string>)[k].trim(), k).not.toBe('');
  });
  it('en and vi use the same {placeholders} for every key', () => {
    const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort().join(',');
    for (const k of Object.keys(vi)) expect(vars((en as Record<string, string>)[k]), k).toBe(vars((vi as Record<string, string>)[k]));
  });
  it('en values contain no Vietnamese diacritics outside the sample glyphs used as examples', () => {
    const VN = /[àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ]/i;
    const ALLOWED = new Set(['onb.art.o1', 'onb.art.o2', 'onb.art.o5', 'onb.art.o6', 'onb.art.o7', 'term.vi_marks.gloss', 'zero.clearVi']); // mô tả chữ Việt minh hoạ
    for (const [k, v] of Object.entries(en)) if (!ALLOWED.has(k)) expect(VN.test(v), `${k}: ${v}`).toBe(false);
  });
});

describe('t() follows the locale', () => {
  it('returns vi by default and en after setLocale; unknown keys still return the key', () => {
    expect(t('settings.title')).toBe('Cài đặt');
    setLocale('en');
    expect(t('settings.title')).toBe('Settings');
    expect(t('sel.count', { n: 3 })).toBe('3 selected');
    expect(t('x.unknown' as I18nKey)).toBe('x.unknown');
    expect(hasKey('settings.title')).toBe(true);
  });
  it('falls back to vi when an en value is missing', () => {
    setLocale('en');
    const saved = (en as Record<string, string>)['settings.title'];
    delete (en as Record<string, string>)['settings.title'];
    try { expect(t('settings.title')).toBe('Cài đặt'); } finally { (en as Record<string, string>)['settings.title'] = saved; }
  });
});

describe('resolving the starting language', () => {
  it('follows the system language when nothing was chosen: vi* → vi, anything else → en', () => {
    expect(localeFromSystem(['vi-VN', 'en'])).toBe('vi');
    expect(localeFromSystem(['en-US', 'vi'])).toBe('en');
    expect(localeFromSystem(['fr-FR'])).toBe('en');
    expect(resolveLocale('vi', null, ['en-US'])).toBe('en');
    expect(resolveLocale('vi', null, ['vi'])).toBe('vi');
  });
  it('an explicit choice wins over the system language; a stored "en" counts as a choice', () => {
    expect(resolveLocale('vi', 'vi', ['en-US'])).toBe('vi');
    expect(resolveLocale('vi', 'en', ['vi'])).toBe('en');
    expect(resolveLocale('en', null, ['vi'])).toBe('en');
  });
  it('chooseLocale applies now, sets <html lang> and remembers the choice', () => {
    chooseLocale('en');
    expect(getLocale()).toBe('en');
    expect(document.documentElement.lang).toBe('en');
    expect(resolveLocale('vi', undefined, ['vi'])).toBe('en');
  });
});

describe('switching re-renders without a restart', () => {
  it('components that call t() during render show the new language; LocaleBoundary remounts the tree', () => {
    const Probe = () => { const l = useLocale(); return <p data-testid="p">{l}:{t('settings.title')}</p>; };
    render(<LocaleBoundary><Probe /></LocaleBoundary>);
    expect(screen.getByTestId('p').textContent).toBe('vi:Cài đặt');
    act(() => setLocale('en'));
    expect(screen.getByTestId('p').textContent).toBe('en:Settings');
    act(() => setLocale('vi'));
    expect(screen.getByTestId('p').textContent).toBe('vi:Cài đặt');
  });
  it('numbers and dates are formatted for the current language', () => {
    expect(formatNumber(1234.5)).toBe('1.234,5');
    expect(formatDate(Date.UTC(2026, 10, 11), { dateStyle: 'medium', timeZone: 'UTC' })).toMatch(/2026/);
    setLocale('en');
    expect(formatNumber(1234.5)).toBe('1,234.5');
    expect(formatDate(Date.UTC(2026, 10, 11), { dateStyle: 'medium', timeZone: 'UTC' })).toBe('Nov 11, 2026');
  });
});
