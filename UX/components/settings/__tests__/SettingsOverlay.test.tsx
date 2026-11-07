import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { Sidebar } from '../../sidebar/Sidebar';
import { SettingsOverlay } from '../SettingsOverlay';

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
});
