import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { SAME_FILE_FAMILY } from '../../../api/mockActivation';
import { useActivationQueue } from '../../../store/activationFlow';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { FaceActionsProvider } from '../../faces/FaceActions';
import { SelectionBar } from '../../main/SelectionBar';
import { Sidebar } from '../../sidebar/Sidebar';
import { ActivationDialogHost } from '../ActivationDialogHost';
import { ActiveRow } from '../ActiveRow';
import { useActivationFlow } from '../useActivationFlow';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
type MockApi = ReturnType<typeof createMockApi>;

function Row({ api, face }: { api: MockApi; face: { hashHex: string; faceIndex: number } | null }) {
  const flow = useActivationFlow(api);
  return <ActiveRow face={face} flow={flow} />;
}
const faceOf = async (api: MockApi, family: string, sub = 'Regular') => {
  const r = (await api.queryFaces({ text: family, limit: 100 })).items.find(x => x.family === family && x.subfamily === sub)!;
  return { row: r, key: { hashHex: r.hashHex, faceIndex: r.faceIndex } };
};

describe('"Có sẵn trên máy" (an identical font is already provided by someone else)', () => {
  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('the peek says who provides it, locks the switch, writes no activation row, and does not toast again on a second press', async () => {
    const api = createMockApi();
    const { key } = await faceOf(api, SAME_FILE_FAMILY);
    render(<Row api={api} face={key} />); await settle();
    const sw = () => screen.getByRole('switch', { name: 'Active' });
    await act(async () => { fireEvent.click(sw()); }); await settle();
    expect(screen.getByTestId('active-status').textContent).toBe('Có sẵn · do FontBase cài');
    expect(sw().getAttribute('aria-disabled')).toBe('true');
    expect(sw().getAttribute('aria-checked')).toBe('false');
    expect(screen.queryByTestId('active-scope')).toBeNull();
    expect(await api.activationState()).toEqual([]); // Font Atlas không ghi gì
    expect(useUiStore.getState().active.size).toBe(0);
    expect(useNoticeStore.getState().notices.map(n => n.text)).toEqual(['1 font đã có sẵn trên máy']);
    await act(async () => { fireEvent.click(sw()); }); await settle();
    expect(useNoticeStore.getState().notices).toHaveLength(1);
  });

  it('recomputed: when the other side removes the font it goes back to "Chưa bật" and the switch works again', async () => {
    const api = createMockApi();
    const { key } = await faceOf(api, SAME_FILE_FAMILY);
    render(<Row api={api} face={key} />); await settle();
    await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); }); await settle();
    expect(useUiStore.getState().available.has(key.hashHex)).toBe(true);
    (api as unknown as { dropAvailable(): void }).dropAvailable();
    await act(async () => { await useUiStore.getState().refreshAvailable(api); });
    expect(screen.getByTestId('active-status').textContent).toBe('Chưa bật');
    expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-disabled')).toBeNull();
  });

  it('the sidebar "Đang bật" counts only fonts Font Atlas turned on, and follows a toggle immediately', async () => {
    const api = createMockApi();
    const inter = await faceOf(api, 'Inter');
    const same = await faceOf(api, SAME_FILE_FAMILY);
    render(<><Sidebar api={api} pickFolder={async () => null} /><Row api={api} face={inter.key} /></>); await settle();
    const count = () => within(screen.getByRole('button', { name: /Đang bật/ })).getByText(/^\d+$/).textContent;
    expect(count()).toBe('0');
    await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); }); await settle();
    expect(screen.getByTestId('active-status').textContent).toBe('Đang bật · phiên này');
    expect(count()).toBe('1');
    await act(async () => { await api.activationActivate([same.key]); await useUiStore.getState().refreshAvailable(api); }); await settle();
    expect(count()).toBe('1'); // bản có sẵn của bên khác không tính
    await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); }); await settle();
    expect(count()).toBe('0');
  });

  it('the selection bar treats available fonts as already satisfied: label is "Tắt kích hoạt" and Activate never re-asks', async () => {
    const api = createMockApi();
    const a = await faceOf(api, 'Inter'); const b = await faceOf(api, SAME_FILE_FAMILY);
    await act(async () => { await api.activationActivate([a.key, b.key]); await useUiStore.getState().refreshActive(api); await useUiStore.getState().refreshAvailable(api); });
    render(<FaceActionsProvider api={api}><SelectionBar rows={[a.row, b.row]} /></FaceActionsProvider>); await settle();
    expect(screen.getByTestId('selbar-activate').textContent).toBe('Tắt kích hoạt');
  });
});

describe('one dialog for the whole app', () => {
  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Two({ api, faces, show }: { api: MockApi; faces: { hashHex: string; faceIndex: number }[]; show: boolean }) {
    const a = useActivationFlow(api); const b = useActivationFlow(api);
    return <>
      <ActivationDialogHost api={api} />
      {show && <button type="button" onClick={() => void a.toggle(faces, true)}>a</button>}
      <button type="button" onClick={() => void b.toggle(faces, true)}>b</button>
    </>;
  }

  it('two callers asking at once never stack two dialogs; the second waits its turn', async () => {
    const api = createMockApi();
    const { key } = await faceOf(api, 'Roboto');
    render(<Two api={api} faces={[key]} show />); await settle();
    await act(async () => { fireEvent.click(screen.getByText('a')); fireEvent.click(screen.getByText('b')); }); await settle();
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Huỷ' })); await settle();
    expect(screen.getAllByRole('dialog')).toHaveLength(1); // yêu cầu thứ hai
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Huỷ' })); await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('the dialog survives its caller leaving the tree (deselecting unmounts the Top Bar shortcut)', async () => {
    const api = createMockApi();
    const { key } = await faceOf(api, 'Roboto');
    const { rerender } = render(<Two api={api} faces={[key]} show />); await settle();
    await act(async () => { fireEvent.click(screen.getByText('a')); }); await settle();
    expect(screen.getByRole('dialog')).toBeTruthy();
    rerender(<Two api={api} faces={[key]} show={false} />); await settle();
    expect(screen.getByRole('dialog')).toBeTruthy();
  });
});
