import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import { BootGate } from '../../components/boot/BootGate';
import { Settings } from '../../components/settings/Settings';
import { createModelStore } from '../../store/model';
import { LocaleBoundary } from '../LocaleBoundary';
import { setLocale } from '..';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('switching language while the app is in use', () => {
  it('Settings keeps its group, DOM node and typed text; only the strings change', async () => {
    const api = createMockApi(); const store = createModelStore();
    await act(async () => { await store.getState().sync(api); });
    render(<LocaleBoundary>{() => <Settings api={api} onClose={() => {}} onRerunOnboarding={() => {}} modelStore={store} />}</LocaleBoundary>);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Dữ liệu' })); await settle();
    const section = screen.getByTestId('settings');
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Dữ liệu');
    act(() => setLocale('en')); await settle();
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Data');
    expect(screen.getByTestId('settings')).toBe(section);
    expect(screen.getByRole('button', { name: 'Back up now' })).toBeTruthy();
  });

  it('BootGate is never remounted: no second boot subscription / status call, children keep their state', async () => {
    const api = createMockApi();
    const subscribe = vi.spyOn(api, 'bootSubscribe'); const status = vi.spyOn(api, 'startupStatus');
    const Child = () => <input data-testid="draft" defaultValue="" />;
    render(<LocaleBoundary>{() => <BootGate api={api}><Child /></BootGate>}</LocaleBoundary>);
    const draft = await screen.findByTestId('draft', {}, { timeout: 8000 });
    fireEvent.change(draft, { target: { value: 'nháp' } });
    const before = { subs: subscribe.mock.calls.length, status: status.mock.calls.length };
    act(() => setLocale('en')); await settle();
    act(() => setLocale('vi')); await settle();
    expect(subscribe.mock.calls.length).toBe(before.subs);
    expect(status.mock.calls.length).toBe(before.status);
    expect(screen.getByTestId('draft')).toBe(draft);
    expect((screen.getByTestId('draft') as HTMLInputElement).value).toBe('nháp');
  });
});
