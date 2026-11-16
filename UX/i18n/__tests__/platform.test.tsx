import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi as vt } from 'vitest';
import { createMockApi } from '../../api/mock';
import { AdvancedFilters } from '../../components/filter/AdvancedFilters';
import { ShortcutsOverlay } from '../../components/help/ShortcutsOverlay';
import { SHORTCUTS } from '../../components/help/shortcuts';
import { WindowShell } from '../../components/shell/WindowShell';
import { useAdvancedSheet } from '../../store/advancedSheet';
import { en } from '../en';
import { EN_ONE, setLocale, t, tn, vi, type I18nKey } from '..';
import { platformText } from '../platformText';
import { detect, getOs, modKey, setOsForTest, shortcutKeys, shortcutText } from '../../platform';

afterEach(() => { setOsForTest('mac'); setLocale('vi'); useAdvancedSheet.setState({ open: false }); });

describe('platform detection', () => {
  it('jsdom (unknown) counts as macOS; forced values win', () => {
    expect(getOs()).toBe('mac');
    setOsForTest('windows'); expect(getOs()).toBe('windows');
    // UA của jsdom trên runner Linux: nếu không ghim thì getOs() sẽ là linux — setup.ts ghim 'mac' cho mọi test.
    expect(detect({ userAgent: 'Mozilla/5.0 (linux) AppleWebKit/537.36 (KHTML, like Gecko) jsdom/26.0.0' })).toBe('linux');
  });
  it('modKey is ⌘ on mac and Ctrl elsewhere', () => {
    expect(modKey({ metaKey: true })).toBe(true); expect(modKey({ ctrlKey: true })).toBe(false);
    setOsForTest('windows'); expect(modKey({ metaKey: true })).toBe(false); expect(modKey({ ctrlKey: true })).toBe(true);
  });
});

describe('shortcut text', () => {
  it('mac is untouched', () => { expect(shortcutText('⌥⌘F')).toBe('⌥⌘F'); expect(shortcutKeys(['⇧', '⌘', 'A'])).toEqual(['⇧', '⌘', 'A']); });
  it('Windows/Linux: Ctrl, Alt, Shift in the usual order', () => {
    setOsForTest('windows');
    expect(shortcutText('⌘K')).toBe('Ctrl+K'); expect(shortcutText('⌘,')).toBe('Ctrl+,'); expect(shortcutText('⌥⌘F')).toBe('Ctrl+Alt+F'); expect(shortcutText('⇧⌘A')).toBe('Ctrl+Shift+A'); expect(shortcutText('⌥G')).toBe('Alt+G'); expect(shortcutText('Esc')).toBe('Esc');
    expect(shortcutKeys(['⌥', '⌘', 'P'])).toEqual(['Ctrl', 'Alt', 'P']);
    expect(shortcutKeys(['⌘', ','])).toEqual(['Ctrl', ',']);
  });
  it('the shortcut cards follow the platform', () => {
    setOsForTest('windows');
    const { container } = render(<ShortcutsOverlay onClose={() => {}} primary="ok" />);
    expect(container.textContent).not.toMatch(/[⌘⌥⇧]/);
    expect(SHORTCUTS.some(s => s.keys.includes('⌘'))).toBe(true); // nguồn vẫn viết bằng ký hiệu mac
  });
  it('Advanced sheet opens with Ctrl+Alt+F on Windows, not ⌘⌥F; on mac only ⌘⌥F', () => {
    render(<AdvancedFilters api={createMockApi()} />);
    fireEvent.keyDown(document, { key: 'f', code: 'KeyF', metaKey: true, altKey: true });
    expect(useAdvancedSheet.getState().open).toBe(true);
    useAdvancedSheet.setState({ open: false });
    fireEvent.keyDown(document, { key: 'f', code: 'KeyF', ctrlKey: true, altKey: true });
    expect(useAdvancedSheet.getState().open).toBe(false);
    setOsForTest('windows');
    fireEvent.keyDown(document, { key: 'f', code: 'KeyF', metaKey: true, altKey: true });
    expect(useAdvancedSheet.getState().open).toBe(false);
    fireEvent.keyDown(document, { key: 'f', code: 'KeyF', ctrlKey: true, altKey: true });
    expect(useAdvancedSheet.getState().open).toBe(true);
  });
});

describe('detect(navigator)', () => {
  it('real WebView tables', () => {
    const mac = { platform: 'MacIntel', userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)', userAgentData: { platform: 'macOS' } };
    const win = { platform: 'Win32', userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36 Edg/126.0', userAgentData: { platform: 'Windows' } };
    const linux = { platform: 'Linux x86_64', userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15' };
    const ipad = { platform: 'MacIntel', userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15' };
    expect(detect(mac)).toBe('mac'); expect(detect(win)).toBe('windows'); expect(detect(linux)).toBe('linux'); expect(detect(ipad)).toBe('mac');
    expect(detect({})).toBe('mac');
    // Không có navigator toàn cục (có điều khiển): không truyền undefined tường minh vì nó kích hoạt tham số mặc định = navigator của jsdom (linux trên runner CI).
    vt.stubGlobal('navigator', undefined); try { expect(detect()).toBe('mac'); } finally { vt.unstubAllGlobals(); }
    expect(detect({ userAgent: 'Mozilla/5.0 (darwin) AppleWebKit/537.36 (KHTML, like Gecko) jsdom/26.0' })).toBe('mac'); // jsdom trên máy mac
  });
});

describe('platform text', () => {
  const ECHO = new Proxy({}, { get: (_, k) => `{${String(k)}}` }) as Record<string, string>; // {biến} → {biến}: t() phải trả đúng mẫu gốc
  it('macOS: t() returns the stored template for EVERY key of the merged dictionaries, vi and en; tn() for every EN_ONE key', () => {
    const keys = Object.keys(vi) as I18nKey[];
    expect(keys.length).toBeGreaterThan(1500);
    for (const loc of ['vi', 'en'] as const) {
      setLocale(loc);
      for (const k of keys) {
        const raw = (loc === 'en' ? (en as Record<string, string>)[k] : undefined) ?? (vi as Record<string, string>)[k];
        expect(t(k, ECHO), `${loc}:${k}`).toBe(raw);
      }
    }
    setLocale('en');
    for (const k of Object.keys(EN_ONE) as I18nKey[]) expect(tn(k, new Proxy({}, { get: (_, v) => (v === 'n' ? 1 : `{${String(v)}}`) }) as Record<string, string>), `one:${k}`).toBe(EN_ONE[k].replace(/\{n\}/g, '1'));
    // và những chuỗi Finder/Trash/Mac thật sự còn nguyên
    expect(t('faces.reveal')).toBe('Show in Finder'); expect(t('dup.d.active')).toBe('Active on this Mac');
    setLocale('vi'); expect(t('faces.reveal')).toBe('Hiện trong Finder'); expect(t('dup.pv.trash')).toBe('Chuyển vào Thùng rác');
  });
  it('the sanity check is not vacuous: the same walk on Windows DOES change strings', () => {
    setOsForTest('windows'); setLocale('en');
    const changed = (Object.keys(vi) as I18nKey[]).filter(k => t(k, ECHO) !== (((en as Record<string, string>)[k]) ?? (vi as Record<string, string>)[k]));
    expect(changed.length).toBeGreaterThan(30);
  });
  it('user data inside {vars} is never rewritten, only the template (t and tn)', () => {
    setOsForTest('windows'); setLocale('en');
    expect(t('faces.finderFail', { msg: 'Trash of Finder on this Mac, macOS' })).toBe('Couldn’t open File Explorer: Trash of Finder on this Mac, macOS');
    expect(tn('act.rec.icloud', { n: 1 })).toBe('1 font is still in iCloud and hasn’t been downloaded.');
    setLocale('vi'); expect(t('faces.finderFail', { msg: 'Finder/macOS' })).toBe('Không mở được File Explorer: Finder/macOS');
  });
  it('“Mac” at the start of a sentence and bare “on Mac” also change on Windows', () => {
    setOsForTest('windows');
    expect(platformText('This Mac is ready. Works on Mac.', 'en')).toBe('This PC is ready. Works on PC.');
    expect(platformText('Chạy trên Mac', 'vi')).toBe('Chạy trên PC');
    expect(platformText('MacBook and macOS', 'en')).toBe('MacBook and Windows');
    expect(platformText('Set the MAC address', 'en')).toBe('Set the MAC address');
  });
  it('Windows: Finder → Explorer, Trash → Recycle Bin, macOS/Mac → Windows/PC (en); Finder → Explorer, macOS → Windows (vi)', () => {
    setOsForTest('windows');
    setLocale('en');
    expect(t('faces.reveal')).toBe('Show in Explorer');
    expect(t('dup.pv.trash')).toBe('Move to Recycle Bin');
    expect(t('err.permission_denied')).toBe('Windows didn’t allow access to this folder.');
    expect(t('dup.d.active')).toBe('Active on this PC');
    expect(t('onb.o5.body')).toContain('never leaves this PC');
    expect(t('faces.finderFail', { msg: 'x' })).toBe('Couldn’t open File Explorer: x');
    setLocale('vi');
    expect(t('faces.reveal')).toBe('Hiện trong Explorer');
    expect(t('err.permission_denied')).toBe('Windows không cho phép đọc thư mục này.');
    expect(t('dup.pv.trash')).toBe('Chuyển vào Thùng rác'); // Windows tiếng Việt cũng gọi là Thùng rác
    expect(t('faces.finderFail', { msg: 'x' })).toBe('Không mở được File Explorer: x');
  });
  it('Linux: generic file manager wording', () => {
    setOsForTest('linux'); setLocale('en');
    expect(t('faces.reveal')).toBe('Show in file manager');
    expect(t('dup.pv.trash')).toBe('Move to Trash');
  });
});

describe('window shell', () => {
  it('mac: titlebar keeps the traffic-light inset marker; Windows/Linux get data-os and never data-native', () => {
    const { rerender } = render(<WindowShell sidebar={null}>x</WindowShell>);
    expect(screen.getByTestId('window-shell').dataset.os).toBe('mac');
    setOsForTest('windows');
    rerender(<WindowShell sidebar={null}>x</WindowShell>);
    expect(screen.getByTestId('window-shell').dataset.os).toBe('windows');
    expect(screen.getByTestId('window-shell').dataset.native).toBe('false');
  });
});
