import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { Sidebar } from '../../sidebar/Sidebar';
import { SettingsOverlay } from '../SettingsOverlay';
import { SettingsShortcut } from '../SettingsShortcut';

const initial = useUiStore.getState();
beforeEach(() => useUiStore.setState(initial, true));

const open = () => { useUiStore.getState().setSettingsOpen(true); return render(<SettingsOverlay api={createMockApi()} onRerunOnboarding={() => {}} />); };

describe('Settings as an overlay', () => {
  it('renders nothing while closed', () => {
    render(<SettingsOverlay api={createMockApi()} onRerunOnboarding={() => {}} />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('is a modal dialog over the content when open', () => {
    open();
    expect(screen.getByRole('dialog', { name: 'Cài đặt' })).toBeTruthy();
    expect(screen.getByTestId('settings')).toBeTruthy();
  });

  it('Esc closes it', () => {
    open();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('clicking outside (the scrim) closes it, clicking inside does not', () => {
    open();
    fireEvent.mouseDown(screen.getByTestId('settings'));
    expect(useUiStore.getState().settingsOpen).toBe(true);
    fireEvent.mouseDown(screen.getByTestId('settings-scrim'));
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('⌘, closes it (toggle) — via the store toggle', () => {
    open();
    useUiStore.getState().toggleSettings();
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('a sidebar item stays clickable and closes it while navigating', async () => {
    const api = createMockApi();
    render(<><Sidebar api={api} pickFolder={async () => null} /><SettingsOverlay api={api} onRerunOnboarding={() => {}} /></>);
    await act(async () => { await new Promise(r => setTimeout(r, 0)); });
    act(() => useUiStore.getState().setSettingsOpen(true));
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Yêu thích/ }));
    expect(useUiStore.getState().settingsOpen).toBe(false);
    expect(useUiStore.getState().view).toBe('star');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('moves focus into the sheet, makes the content below inert, and restores focus on close', () => {
    const { container } = render(<div><main><button>outside</button></main><SettingsOverlay api={createMockApi()} onRerunOnboarding={() => {}} /></div>);
    const outside = screen.getByText('outside'); outside.focus();
    act(() => useUiStore.getState().setSettingsOpen(true));
    expect(screen.getByRole('dialog').contains(document.activeElement)).toBe(true);
    expect(container.querySelector('main')!.inert).toBe(true);
    expect(screen.getByRole('dialog').getAttribute('aria-modal')).toBeNull();
    act(() => useUiStore.getState().setSettingsOpen(false));
    expect(container.querySelector('main')!.inert).toBe(false);
    expect(document.activeElement).toBe(outside);
  });

  it('Esc is left to an open context menu or confirm dialog (does not also close Settings)', () => {
    open();
    const menu = document.createElement('div'); menu.setAttribute('role', 'menu'); document.body.append(menu);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().settingsOpen).toBe(true);
    menu.remove();
    const modal = document.createElement('div'); modal.setAttribute('aria-modal', 'true'); document.body.append(modal);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().settingsOpen).toBe(true);
    modal.remove();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('⌘, through the real shortcut opens and closes the overlay', () => {
    render(<><SettingsShortcut /><SettingsOverlay api={createMockApi()} onRerunOnboarding={() => {}} /></>);
    fireEvent.keyDown(document.body, { key: ',', metaKey: true });
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.keyDown(document.body, { key: ',', metaKey: true });
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
