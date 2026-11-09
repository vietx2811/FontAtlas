import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { ActiveRow } from '../ActiveRow';
import { useActivationFlow } from '../useActivationFlow';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
type MockApi = ReturnType<typeof createMockApi>;

function Harness({ api, face }: { api: MockApi; face: { hashHex: string; faceIndex: number } | null }) {
  const flow = useActivationFlow(api);
  return <>{flow.dialog}<ActiveRow face={face} flow={flow} /></>;
}
async function setup() {
  const api = createMockApi();
  const r = (await api.queryFaces({ text: 'Inter', limit: 10 })).items[0];
  const face = { hashHex: r.hashHex, faceIndex: r.faceIndex };
  render(<Harness api={api} face={face} />);
  await settle();
  return { api, face };
}
const sw = () => screen.getByRole('switch', { name: 'Active' });

describe('ActiveRow', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('shows a LABELLED switch whose label is a TermTip term; off by default, no scope button', async () => {
    await setup();
    expect(sw().getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText('Active').getAttribute('data-term')).toBe('activate');
    expect(screen.getByTestId('active-status').textContent).toBe('Chưa bật');
    expect(screen.queryByTestId('active-scope')).toBeNull();
  });

  it('on → "Đang bật · phiên này"; the scope button flips to "Luôn bật" and back, staying active', async () => {
    const { api, face } = await setup();
    await act(async () => { fireEvent.click(sw()); }); await settle();
    expect(sw().getAttribute('aria-checked')).toBe('true');
    expect(screen.getByTestId('active-status').textContent).toBe('Đang bật · phiên này');
    expect(screen.getByTestId('active-scope').textContent).toBe('Giữ bật');
    await act(async () => { fireEvent.click(screen.getByTestId('active-scope')); }); await settle();
    expect(screen.getByTestId('active-status').textContent).toBe('Luôn bật');
    expect(screen.getByTestId('active-scope').textContent).toBe('Chỉ phiên này');
    expect((await api.activationState()).find(s => s.hashHex === face.hashHex)).toMatchObject({ state: 'active', scope: 'persistent' });
    await act(async () => { fireEvent.click(screen.getByTestId('active-scope')); }); await settle();
    expect(screen.getByTestId('active-status').textContent).toBe('Đang bật · phiên này');
    expect(sw().getAttribute('aria-checked')).toBe('true');
  });

  it('off clears the status and removes the scope button', async () => {
    await setup();
    await act(async () => { fireEvent.click(sw()); }); await settle();
    await act(async () => { fireEvent.click(sw()); }); await settle();
    expect(sw().getAttribute('aria-checked')).toBe('false');
    expect(screen.getByTestId('active-status').textContent).toBe('Chưa bật');
    expect(screen.queryByTestId('active-scope')).toBeNull();
  });

  it('without a face (still loading) the switch is disabled and does nothing', async () => {
    const api = createMockApi();
    render(<Harness api={api} face={null} />); await settle();
    expect(sw().getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(sw()); await settle();
    expect(await api.activationState()).toEqual([]);
  });
});
