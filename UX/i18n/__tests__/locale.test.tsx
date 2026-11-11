import { act, fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it, vi as vitest } from 'vitest';
import { chooseLocale, formatDate, formatNumber, getLocale, hasChosenThisSession, hasKey, resolveLocale, setLocale, t, tn, vi, EN_ONE, type I18nKey } from '..';
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
    const ALLOWED = new Set(['onb.art.o1', 'onb.art.o2', 'onb.art.o5', 'onb.art.o6', 'onb.art.o7', 'term.vi_marks.gloss', 'zero.clearVi', 'dup.sampleText']); // mô tả chữ Việt minh hoạ
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
  it('uses the saved uiLang; when none is saved (null) it follows the system language: vi* → vi, anything else → en', () => {
    expect(resolveLocale('en', ['vi'])).toBe('en');
    expect(resolveLocale('vi', ['en-US'])).toBe('vi');
    expect(resolveLocale(null, ['vi-VN', 'en'])).toBe('vi');
    expect(resolveLocale(null, ['en-US', 'vi'])).toBe('en');
    expect(resolveLocale(undefined, ['fr-FR'])).toBe('en');
    expect(localeFromSystem(['vi'])).toBe('vi');
  });
  it('chooseLocale applies now, sets <html lang> and marks the session as chosen', () => {
    expect(hasChosenThisSession()).toBe(false);
    chooseLocale('en');
    expect(getLocale()).toBe('en');
    expect(document.documentElement.lang).toBe('en');
    expect(hasChosenThisSession()).toBe(true);
  });
});

describe('switching re-renders without a restart', () => {
  it('components that call t() during render show the new language and KEEP their state (no remount)', () => {
    const Probe = () => { const [n, setN] = useState(0); return <button data-testid="p" onClick={() => setN(n + 1)}>{n}:{t('settings.title')}</button>; };
    render(<LocaleBoundary>{() => <Probe />}</LocaleBoundary>);
    fireEvent.click(screen.getByTestId('p')); fireEvent.click(screen.getByTestId('p'));
    expect(screen.getByTestId('p').textContent).toBe('2:Cài đặt');
    const node = screen.getByTestId('p');
    act(() => setLocale('en'));
    expect(screen.getByTestId('p').textContent).toBe('2:Settings');
    expect(screen.getByTestId('p')).toBe(node); // cùng một DOM node: không bị dựng lại
    act(() => setLocale('vi'));
    expect(screen.getByTestId('p').textContent).toBe('2:Cài đặt');
  });
  it('numbers and dates are formatted for the current language', () => {
    expect(formatNumber(1234.5)).toBe('1.234,5');
    expect(formatDate(Date.UTC(2026, 10, 11), { dateStyle: 'medium', timeZone: 'UTC' })).toMatch(/2026/);
    setLocale('en');
    expect(formatNumber(1234.5)).toBe('1,234.5');
    expect(formatDate(Date.UTC(2026, 10, 11), { dateStyle: 'medium', timeZone: 'UTC' })).toBe('Nov 11, 2026');
  });
});

describe('backend error messages', () => {
  it('Vietnamese UI keeps the backend sentence; English UI maps the code to an English sentence', async () => {
    const { toCmdError } = await import('../../api/tauri');
    const raw = { code: 'not_found', message: 'không có face này trong Index' };
    expect(toCmdError(raw).message).toBe('không có face này trong Index');
    setLocale('en');
    expect(toCmdError(raw).message).toBe('Couldn’t find that item.');
    expect(toCmdError({ code: 'weird_new_code', message: 'lỗi lạ' }).message).toBe('Something went wrong (weird_new_code).');
  });
});

describe('key hygiene', () => {
  it('no key is declared in two groups (a later spread would silently override the earlier one)', async () => {
    const mods = await Promise.all(['activation', 'cloud', 'compareV2', 'desk', 'filter', 'onboarding', 'terms', 'view', 'sidebar', 'faces', 'ui', 'detail', 'settings', 'err', 'dup'].map(n => import(`../${n}.ts`)));
    const { MAIN_I18N } = await import('..');
    const seen = new Map<string, string>(); const dup: string[] = [];
    const groups: Array<[string, object]> = [['main', MAIN_I18N]];
    for (const [i, m] of mods.entries()) for (const [name, v] of Object.entries(m)) if (v && typeof v === 'object') groups.push([`${i}:${name}`, v as object]);
    for (const [g, o] of groups) for (const k of Object.keys(o)) { if (seen.has(k)) dup.push(`${k} in ${seen.get(k)} and ${g}`); else seen.set(k, g); }
    expect(dup).toEqual([]);
    expect(seen.size).toBe(Object.keys(vi).length);
  });
  it('a missing dynamic key returns the key and warns once in dev', () => {
    const warn = vitest.spyOn(console, 'warn').mockImplementation(() => {});
    expect(t('class.__nope__' as I18nKey)).toBe('class.__nope__');
    expect(t('class.__nope__' as I18nKey)).toBe('class.__nope__');
    expect(warn).toHaveBeenCalledTimes(1);
    expect(String(warn.mock.calls[0][0])).toContain('class.__nope__');
    warn.mockRestore();
  });
  it('tn picks the English singular for n = 1 only; Vietnamese has a single form', () => {
    expect(tn('faces.nFonts', { n: 1 })).toBe('1 font');
    setLocale('en');
    expect(tn('faces.nFonts', { n: 1 })).toBe('1 font');
    expect(tn('faces.nFonts', { n: 2 })).toBe('2 fonts');
    expect(tn('faces.nFonts', { n: '1' })).toBe('1 font');
    expect(tn('faces.nFonts', { n: formatNumber(1234) })).toBe('1,234 fonts');
  });
  it('every singular form (EN_ONE) matches a key and its placeholders', () => {
    const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort().join(',');
    for (const [k, one] of Object.entries(EN_ONE)) { expect(hasKey(k), k).toBe(true); expect(vars(one), k).toBe(vars((en as Record<string, string>)[k])); }
  });
});
