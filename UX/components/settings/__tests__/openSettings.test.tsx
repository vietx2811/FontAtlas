import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { Sidebar } from '../../sidebar/Sidebar';
import { SettingsShortcut } from '../SettingsShortcut';

const initial = useUiStore.getState();
beforeEach(() => useUiStore.setState(initial, true));

describe('opening Settings', () => {
  it('the gear in the sidebar opens it (it used to do nothing)', async () => {
    render(<Sidebar api={createMockApi()} pickFolder={async () => null} />);
    await act(async () => { await new Promise(r => setTimeout(r, 0)); });
    expect(useUiStore.getState().settingsOpen).toBe(false);
    fireEvent.click(screen.getByRole('button', { name: 'Cài đặt' }));
    expect(useUiStore.getState().settingsOpen).toBe(true);
  });

  it('⌘, toggles it from anywhere', () => {
    render(<SettingsShortcut />);
    fireEvent.keyDown(document.body, { key: ',', metaKey: true });
    expect(useUiStore.getState().settingsOpen).toBe(true);
    fireEvent.keyDown(document.body, { key: ',', metaKey: true });
    expect(useUiStore.getState().settingsOpen).toBe(false);
    fireEvent.keyDown(document.body, { key: ',', ctrlKey: true });
    expect(useUiStore.getState().settingsOpen).toBe(true);
  });
});
