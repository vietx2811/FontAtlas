import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { faceKey, useUiStore } from '../../../store/ui';
import { DetailPanel } from '../DetailPanel';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
async function select(api: ReturnType<typeof createMockApi>, family: string, sub?: string) {
  const r = (await api.queryFaces({ text: family, limit: 100 })).items.find(x => !sub || x.subfamily === sub)!;
  act(() => useUiStore.getState().select(faceKey(r)));
  return r;
}

describe('DetailPanel', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('renders nothing until a face is selected', async () => {
    const { container } = render(<DetailPanel api={createMockApi()} />);
    await settle();
    expect(container.firstChild).toBeNull();
  });

  it('shows name, summary line and the style dropdown for the selected face', async () => {
    const api = createMockApi();
    render(<DetailPanel api={api} />);
    await select(api, 'Public Sans', 'Regular');
    await settle();
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Public Sans');
    expect(screen.getByTestId('panel-sub').textContent).toMatch(/TTF · [\d.,]+ glyph/);
    expect(screen.getByRole('button', { name: 'Kiểu chữ' }).textContent).toContain('Regular');
    expect(screen.getByRole('button', { name: 'Kiểu chữ' }).textContent).toContain('18 kiểu');
  });

  it('choosing another style re-reads the face and keeps every tab on that style', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'getFace');
    render(<DetailPanel api={api} />);
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
    render(<DetailPanel api={api} />);
    await select(api, 'Pacifico');
    await settle();
    const dd = screen.getByRole('button', { name: 'Kiểu chữ' }) as HTMLButtonElement;
    expect(dd.disabled).toBe(true);
    expect(dd.textContent).toContain('Regular');
  });

  it('the switch reflects and drives activation', async () => {
    const api = createMockApi();
    render(<DetailPanel api={api} />);
    const r = await select(api, 'Inter', 'Regular');
    await settle();
    const sw = screen.getByRole('switch', { name: 'Kích hoạt font' });
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
    render(<DetailPanel api={api} />);
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
    render(<DetailPanel api={api} />);
    await select(api, 'Inter', 'Regular');
    await settle();
    expect(within(screen.getByRole('alert')).getByText(/Không đọc được font/)).toBeTruthy();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Thử lại' })); });
    await settle();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Inter');
  });
});
