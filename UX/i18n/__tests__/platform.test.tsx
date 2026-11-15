import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../api/mock';
import { AdvancedFilters } from '../../components/filter/AdvancedFilters';
import { ShortcutsOverlay } from '../../components/help/ShortcutsOverlay';
import { SHORTCUTS } from '../../components/help/shortcuts';
import { WindowShell } from '../../components/shell/WindowShell';
import { useAdvancedSheet } from '../../store/advancedSheet';
import { en } from '../en';
import { MAIN_I18N, setLocale, t, tn } from '..';
import { platformText } from '../platformText';
import { getOs, modKey, setOsForTest, shortcutKeys, shortcutText } from '../../platform';

afterEach(() => { setOsForTest(null); setLocale('vi'); useAdvancedSheet.setState({ open: false }); });

describe('platform detection', () => {
  it('jsdom (unknown) counts as macOS; forced values win', () => {
    expect(getOs()).toBe('mac');
    setOsForTest('windows'); expect(getOs()).toBe('windows');
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

describe('platform text', () => {
  it('macOS: every vi and en string is returned exactly as written', () => {
    for (const s of [...Object.values(MAIN_I18N), ...Object.values(en as Record<string, string>)]) expect(platformText(s, 'en')).toBe(s);
    setLocale('en'); expect(t('faces.reveal')).toBe('Show in Finder'); expect(tn('act.rec.icloud', { n: 1 })).toContain('iCloud');
    setLocale('vi'); expect(t('faces.reveal')).toBe('Hiện trong Finder'); expect(t('dup.pv.trash')).toBe('Chuyển vào Thùng rác');
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
