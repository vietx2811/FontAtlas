import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { PairingDesk } from '../PairingDesk';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const wait = (ms: number) => act(async () => { await new Promise(r => setTimeout(r, ms)); });
async function mount(fill = true) {
  const api = createMockApi();
  const r = (await api.queryFaces({ limit: 200 })).items.find(x => x.family === 'Playfair Display')!;
  const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
  let comboId = res.comboId;
  if (!fill) comboId = (await api.createCombo(res.case.id)).id;
  render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId, role: 'body', trial: null }} />); await settle();
  return { api, res, comboId };
}
const type = async (text: string) => {
  const input = screen.getByTestId('combo-name') as HTMLInputElement;
  fireEvent.focus(input); fireEvent.change(input, { target: { value: text } });
  await wait(350); await settle(); // debounce 300 ms
  return input;
};

describe('G6 gợi ý combo', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it('focus ô tên mở popover; gõ "Editorial" → "Hiểu là…" và ≥ 3 gợi ý bằng chữ thật; tên lạ → "Chưa nhận ra" vẫn có gợi ý', async () => {
    await mount();
    await type('Editorial');
    expect(screen.getByRole('dialog', { name: 'Gợi ý combo' })).toBeTruthy();
    expect(screen.getByTestId('cs-intent').textContent).toMatch(/Hiểu là: Editorial, Tạp chí/);
    expect(screen.getAllByTestId('cs-card').length).toBeGreaterThanOrEqual(3);
    await type('Dự án 7');
    expect(screen.getByTestId('cs-intent').textContent).toMatch(/Chưa nhận ra ý đồ/);
    expect(screen.getAllByTestId('cs-card').length).toBeGreaterThan(0);
  });

  it('Nhận cả combo khi combo đã có font → combo MỚI "<tên> 2", chuyển sang đó, combo cũ nguyên; Hoàn tác xoá combo mới', async () => {
    const { api, res, comboId } = await mount();
    const before = (await api.getCase(res.case.id)).combos[0].slots.map(s => `${s.role}:${s.key.hashHex}`);
    await type('Editorial');
    fireEvent.click(screen.getAllByTestId('cs-take-all')[0]); await settle();
    const c = await api.getCase(res.case.id);
    expect(c.combos).toHaveLength(2);
    const made = c.combos.find(x => x.id !== comboId)!;
    expect(made.name).toBe('Editorial 2'); expect(made.slots.length).toBeGreaterThanOrEqual(2);
    expect(c.combos.find(x => x.id === comboId)!.slots.map(s => `${s.role}:${s.key.hashHex}`)).toEqual(before);
    expect(useUiStore.getState().desk?.comboId).toBe(made.id);
    const n = useNoticeStore.getState().notices.at(-1)!; expect(n.text).toMatch(/Đã tạo Editorial 2/); expect(n.action?.label).toBe('Hoàn tác');
    n.action!.run(); await settle();
    expect((await api.getCase(res.case.id)).combos).toHaveLength(1);
  });

  it('Nhận cả combo khi combo đang rỗng → điền tại chỗ (không tạo combo); Hoàn tác gỡ các font đã điền', async () => {
    const { api, res, comboId } = await mount(false);
    await type('Poster');
    fireEvent.click(screen.getAllByTestId('cs-take-all')[0]); await settle();
    const c = await api.getCase(res.case.id);
    expect(c.combos).toHaveLength(2); // combo rỗng thêm trong test + combo gốc, không có combo thứ ba
    expect(c.combos.find(x => x.id === comboId)!.slots.length).toBeGreaterThanOrEqual(2);
    useNoticeStore.getState().notices.at(-1)!.action!.run(); await settle();
    expect((await api.getCase(res.case.id)).combos.find(x => x.id === comboId)!.slots).toHaveLength(0);
  });

  it('＋ ở một vai đặt đúng vai của combo đang mở; Hoàn tác trả lại font cũ', async () => {
    const { api, res, comboId } = await mount();
    await type('Editorial');
    const before = (await api.getCase(res.case.id)).combos.find(x => x.id === comboId)!;
    const btn = screen.getAllByTestId('cs-take-role')[0] as HTMLButtonElement;
    const role = btn.getAttribute('aria-label')!.replace(/^Nhận vai /, '').toLowerCase();
    fireEvent.click(btn); await settle();
    const after = (await api.getCase(res.case.id)).combos.find(x => x.id === comboId)!;
    expect(after.slots.some(s => s.role.toLowerCase() === role)).toBe(true);
    useNoticeStore.getState().notices.at(-1)!.action!.run(); await settle();
    const undone = (await api.getCase(res.case.id)).combos.find(x => x.id === comboId)!;
    expect(undone.slots.map(s => `${s.role}:${s.key.hashHex}`).sort()).toEqual(before.slots.map(s => `${s.role}:${s.key.hashHex}`).sort());
  });

  it('Giữ font đã đặt: vai đã có font hiện khoá 🔒 và không có nút ＋; L bỏ khoá', async () => {
    const { desk } = await mountWithDesk();
    await type('Editorial');
    expect(screen.getAllByTestId('cs-card')[0].textContent).toMatch(/🔒/);
    fireEvent.keyDown(screen.getByTestId('combo-suggest'), { key: 'l' }); await wait(30); await settle();
    expect((screen.getByTestId('cs-keep') as HTMLInputElement).checked).toBe(false);
    void desk;
  });

  it('Esc đóng popover và giữ tên đã gõ; N ở Desk focus ô tên; R đổi seed (gọi lại)', async () => {
    const { api } = await mount(); const spy = vi.spyOn(api, 'suggestCombos');
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'n' }); await settle();
    expect(document.activeElement).toBe(screen.getByTestId('combo-name'));
    await type('Brand');
    const calls = spy.mock.calls.length;
    fireEvent.keyDown(screen.getByTestId('combo-suggest'), { key: 'r' }); await wait(30); await settle();
    expect(spy.mock.calls.length).toBeGreaterThan(calls);
    fireEvent.keyDown(screen.getByTestId('combo-name'), { key: 'Escape' }); await settle();
    expect(screen.queryByTestId('combo-suggest')).toBeNull(); expect((screen.getByTestId('combo-name') as HTMLInputElement).value).toBe('Brand');
    expect(screen.getByTestId('pairing-desk')).toBeTruthy(); // Desk không bị đóng bởi Esc của ô tên
  });

  it('lỗi backend hiện trong popover với Thử lại; tên không mất', async () => {
    const { api } = await mount();
    const spy = vi.spyOn(api, 'suggestCombos').mockRejectedValue(new Error('hỏng'));
    await type('Menu');
    expect(screen.getByRole('alert').textContent).toMatch(/hỏng/);
    spy.mockRestore();
    fireEvent.click(screen.getByTestId('cs-retry')); await wait(30); await settle();
    expect(screen.queryByRole('alert')).toBeNull(); expect((screen.getByTestId('combo-name') as HTMLInputElement).value).toBe('Menu');
  });

  it('Enter lưu tên combo qua update_combo', async () => {
    const { api, res, comboId } = await mount();
    const input = await type('Tạp chí bếp'); fireEvent.keyDown(input, { key: 'Enter' }); await settle();
    expect((await api.getCase(res.case.id)).combos.find(x => x.id === comboId)!.name).toBe('Tạp chí bếp');
  });
});

async function mountWithDesk() { return { desk: null as unknown, ...(await mount()) }; }
