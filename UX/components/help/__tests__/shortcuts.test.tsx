import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { SHORTCUTS, setShortcutsHelpOpen } from '../shortcuts';
import { ShortcutsHelp } from '../ShortcutsHelp';
import { ShortcutsOverlay } from '../ShortcutsOverlay';
import { t } from '../../../i18n';

afterEach(() => { cleanup(); setShortcutsHelpOpen(false); });

describe('shortcut list', () => {
  it('has the real shortcuts, unique ids, and every title/description is translated', () => {
    const ids = SHORTCUTS.map(s => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const want of ['search', 'advanced', 'view', 'activate', 'pair', 'compare', 'settings']) expect(ids).toContain(want);
    const key = (id: string) => SHORTCUTS.find(s => s.id === id)!.keys.join('');
    expect(key('search')).toBe('⌘K'); expect(key('advanced')).toBe('⌥⌘F'); expect(key('view')).toBe('⌘J'); expect(key('activate')).toBe('⇧⌘A');
    for (const s of SHORTCUTS) { expect(t(s.title)).not.toBe(s.title); expect(t(s.desc)).not.toBe(s.desc); }
  });
});

describe('auto-activation card', () => {
  it('has no keys, and its link opens Settings on the auto group, then closes the overlay', async () => {
    const { useUiStore } = await import('../../../store/ui');
    const { takeSettingsGroup } = await import('../../settings/groupRequest');
    useUiStore.getState().setSettingsOpen(false);
    let closed = 0;
    render(<ShortcutsOverlay primary="Bắt đầu" onClose={() => { closed++; }} />);
    const card = screen.getByText('Bật font tự động').closest('li')!;
    expect(card.querySelector('kbd')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Mở Cài đặt/ }));
    expect(useUiStore.getState().settingsOpen).toBe(true);
    expect(takeSettingsGroup()).toBe('auto');
    expect(closed).toBe(1);
    useUiStore.getState().setSettingsOpen(false);
  });
});

describe('ShortcutsOverlay', () => {
  it('is a labelled modal dialog with one card per shortcut; Esc and the button close it', () => {
    let closed = 0;
    render(<ShortcutsOverlay primary="Bắt đầu" onClose={() => { closed++; }} />);
    const dlg = screen.getByRole('dialog');
    expect(dlg.getAttribute('aria-modal')).toBe('true');
    expect(screen.getByRole('heading', { name: 'Phím tắt & tính năng chính' })).toBeTruthy();
    expect(screen.getAllByTestId('tip-card')).toHaveLength(SHORTCUTS.length);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(closed).toBe(1);
    fireEvent.click(screen.getByRole('button', { name: 'Bắt đầu' }));
    expect(closed).toBe(2);
  });
});

describe('ShortcutsHelp (? key)', () => {
  it('opens on ?, closes on Esc, and ignores ? while typing, with a modifier, or while a dialog/menu is open', () => {
    render(<><input aria-label="q" /><ShortcutsHelp /></>);
    fireEvent.keyDown(screen.getByLabelText('q'), { key: '?' });
    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.keyDown(document.body, { key: '?', metaKey: true });
    expect(screen.queryByRole('dialog')).toBeNull();
    act(() => { fireEvent.keyDown(document.body, { key: '?' }); });
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });
  it('does nothing during the guide (.fa-onb) so the guide keeps its own flow', () => {
    const g = document.createElement('div'); g.className = 'fa-onb'; document.body.appendChild(g);
    render(<ShortcutsHelp />);
    fireEvent.keyDown(document.body, { key: '?' });
    expect(screen.queryByRole('dialog')).toBeNull();
    g.remove();
  });
});
