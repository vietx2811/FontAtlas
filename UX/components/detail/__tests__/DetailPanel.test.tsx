import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { faceKey, useUiStore } from '../../../store/ui';
import { DetailPanel } from '../DetailPanel';
import { ActivationDialogHost } from '../../activation/ActivationDialogHost';
import { useActivationQueue } from '../../../store/activationFlow';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
async function select(api: ReturnType<typeof createMockApi>, family: string, sub?: string) {
  const r = (await api.queryFaces({ text: family, limit: 100 })).items.find(x => !sub || x.subfamily === sub)!;
  act(() => useUiStore.getState().select(faceKey(r)));
  return r;
}

describe('DetailPanel', () => {
  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('renders nothing until a face is selected', async () => {
    const { container } = render(<DetailPanel api={createMockApi()} />);
    await settle();
    expect(container.firstChild).toBeNull();
  });

  it('shows name, summary line and the style dropdown for the selected face', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Public Sans');
    expect(screen.getByTestId('panel-sub').textContent).toMatch(/TTF · [\d.,]+ glyph/);
    expect(screen.getByRole('button', { name: 'Kiểu chữ' }).textContent).toContain('Regular');
    expect(screen.getByRole('button', { name: 'Kiểu chữ' }).textContent).toContain('18 kiểu');
  });

  it('marks a face that is only a cloud placeholder in the header', async () => {
    const api = createMockApi();
    const real = api.getFace.bind(api);
    vi.spyOn(api, 'getFace').mockImplementation(async (...args: Parameters<typeof real>) => {
      const d = await real(...args);
      return { ...d, row: { ...d.row, onCloudOnly: true } };
    });
    render(<DetailPanel api={api} />);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    expect(screen.getByTestId('cloud-only-note')).toBeTruthy();
  });

  it('shows no cloud note for a font that is on this machine', async () => {
    const api = createMockApi();
    render(<DetailPanel api={api} />);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    expect(screen.queryByTestId('cloud-only-note')).toBeNull();
  });

  it('choosing another style re-reads the face and keeps every tab on that style', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'getFace');
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Kiểu chữ' }));
    fireEvent.click(screen.getByRole('option', { name: 'Bold' }));
    await settle();
    expect(useUiStore.getState().style).toBe('Bold');
    expect(screen.getByRole('button', { name: 'Kiểu chữ' }).textContent).toContain('Bold');
    expect(spy.mock.calls.at(-1)![0]).toEqual(expect.objectContaining({ hashHex: expect.any(String) }));
    expect(screen.getByRole('tabpanel').getAttribute('data-style')).toBe('Bold');
    fireEvent.click(screen.getByRole('tab', { name: 'Tiếng Việt' }));
    expect(screen.getByRole('tabpanel').getAttribute('data-style')).toBe('Bold');
  });

  it('a single-style family has a disabled dropdown that still names the style', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Pacifico');
    await settle();
    const dd = screen.getByRole('button', { name: 'Kiểu chữ' }) as HTMLButtonElement;
    expect(dd.disabled).toBe(true);
    expect(dd.textContent).toContain('Regular');
  });

  it('the switch reflects and drives activation', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    const r = await select(api, 'Inter', 'Regular');
    await settle();
    const sw = screen.getByRole('switch', { name: 'Active' });
    expect(sw.getAttribute('aria-checked')).toBe('false');
    await act(async () => { fireEvent.click(sw); });
    await settle();
    expect(sw.getAttribute('aria-checked')).toBe('true');
    expect((await api.activationState()).some(s => s.hashHex === r.hashHex && s.state === 'active')).toBe(true);
    await act(async () => { fireEvent.click(sw); });
    await settle();
    expect(sw.getAttribute('aria-checked')).toBe('false');
  });

  it('info card toggle persists in the store; five sticky tabs are present', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Roboto', 'Regular');
    await settle();
    fireEvent.click(screen.getByRole('button', { name: /Hãng · Phiên bản/ }));
    expect(useUiStore.getState().infoExpanded).toBe(true);
    expect(screen.getAllByRole('tab').map(t => t.getAttribute('aria-label'))).toEqual(['Specimen', 'Tiếng Việt', 'Giải phẫu', 'Tương tự', 'Ghép cặp']);
    fireEvent.click(screen.getByRole('tab', { name: 'Giải phẫu' }));
    expect(useUiStore.getState().panelTab).toBe('anat');
  });

  it('shows an error message with retry when the face cannot be read', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'getFace').mockRejectedValueOnce(new Error('boom'));
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Inter', 'Regular');
    await settle();
    expect(within(screen.getByRole('alert')).getByText(/Không đọc được font/)).toBeTruthy();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Thử lại' })); });
    await settle();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Inter');
  });

  describe('activation flow (Plan 7 contract)', () => {
    it('a clash with a system font opens D3; "Vẫn kích hoạt" is not allowed for system fonts; Huỷ leaves it off', async () => {
      const api = createMockApi();
      render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
      await select(api, 'Roboto', 'Regular');
      await settle();
      await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); });
      await settle();
      const dlg = screen.getByRole('dialog', { name: /đã có trên máy/ });
      expect(dlg.textContent).toMatch(/Roboto · 1 kiểu/);
      expect(within(dlg).getByRole('img', { name: /của hệ thống · \/Library\/Fonts\/Roboto\.ttf/ })).toBeTruthy(); // owner + đường dẫn chỉ ở tooltip
      expect((within(dlg).getByRole('radio', { name: /Vẫn kích hoạt/ }) as HTMLInputElement).disabled).toBe(true);
      expect(dlg.textContent).toMatch(/font hệ thống luôn được ưu tiên/);
      fireEvent.click(within(dlg).getByRole('button', { name: 'Huỷ' }));
      expect(screen.queryByRole('dialog')).toBeNull();
      expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-checked')).toBe('false');
    });

    it('a clash with another manager lets you proceed anyway', async () => {
      const api = createMockApi();
      render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
      await select(api, 'Oswald', 'Regular');
      await settle();
      await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); });
      await settle();
      const dlg = screen.getByRole('dialog');
      expect(within(dlg).getByRole('img', { name: /do RightFont cài/ })).toBeTruthy();
      expect(within(dlg).getByRole('list').textContent).not.toMatch(/RightFont/); // danh sách không in inline; tên app chỉ ở dòng cảnh báo thay thế
      expect(within(dlg).getByTestId('replace-warning').textContent).toMatch(/RightFont/);
      expect((within(dlg).getByRole('radio', { name: /Thay bằng bản này/ }) as HTMLInputElement).checked).toBe(true); // RightFont, không app lạ → chọn sẵn thay
      fireEvent.click(within(dlg).getByRole('radio', { name: /Vẫn kích hoạt/ }));
      await act(async () => { fireEvent.click(within(dlg).getByRole('button', { name: 'Tiếp tục' })); });
      await settle();
      expect(screen.queryByRole('dialog')).toBeNull();
      expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-checked')).toBe('true');
    });

    it('a file changed on disk shows an error notice and nothing turns on', async () => {
      const api = createMockApi();
      render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
      await select(api, 'Lobster', 'Regular');
      await settle();
      await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); });
      await settle();
      const n = useNoticeStore.getState().notices;
      expect(n).toHaveLength(1);
      expect(n[0]).toMatchObject({ kind: 'error' });
      expect(n[0].text).toMatch(/đã thay đổi trên đĩa/);
      expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-checked')).toBe('false');
    });

    it('turning off a font held by a project asks first, then forces', async () => {
      const api = createMockApi();
      const r = await select(api, 'Inter', 'Regular');
      const col = await api.createCollection('Menu', null);
      await api.addToCollection(col.id, [{ hashHex: r.hashHex, faceIndex: 0 }]);
      await api.activationActivateCollection(col.id);
      render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
      await act(async () => { await useUiStore.getState().refreshActive(api); });
      await settle();
      const sw = screen.getByRole('switch', { name: 'Active' });
      expect(sw.getAttribute('aria-checked')).toBe('true');
      await act(async () => { fireEvent.click(sw); });
      await settle();
      const dlg = screen.getByRole('dialog', { name: /đang được giữ bởi 1 dự án/ });
      await act(async () => { fireEvent.click(within(dlg).getByRole('button', { name: 'Tắt luôn' })); });
      await settle();
      expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-checked')).toBe('false');
      expect(await api.activationState()).toEqual([]);
    });
  });

  it('family button: turns every style of the family on/off at once; pressed only when all styles are on', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    const btn = () => screen.getByTestId('family-active');
    expect(btn().getAttribute('aria-pressed')).toBe('false');
    await act(async () => { fireEvent.click(btn()); }); await settle();
    const hashes = new Set((await api.activationState()).map(s => s.hashHex));
    expect(hashes.size).toBeGreaterThan(5); // cả family, không chỉ kiểu đang xem
    expect(btn().getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('switch', { name: 'Active' }).getAttribute('aria-checked')).toBe('true');
    await act(async () => { fireEvent.click(btn()); }); await settle();
    expect(await api.activationState()).toEqual([]);
    expect(btn().getAttribute('aria-pressed')).toBe('false');
  });

  it('family button is only "on" when ALL styles are active (one style on is not enough)', async () => {
    const api = createMockApi();
    render(<><DetailPanel api={api} /><ActivationDialogHost api={api} /></>);
    const r = await select(api, 'Public Sans', 'Regular');
    await settle();
    await act(async () => { fireEvent.click(screen.getByRole('switch', { name: 'Active' })); }); await settle();
    expect((await api.activationState()).length).toBe(1);
    expect(screen.getByTestId('family-active').getAttribute('aria-pressed')).toBe('false');
    expect(r).toBeTruthy();
  });
});
