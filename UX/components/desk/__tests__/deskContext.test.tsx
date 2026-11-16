import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { PairingDesk } from '../PairingDesk';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
async function mount() {
  const api = createMockApi();
  const r = (await api.queryFaces({ limit: 200 })).items.find(x => x.family === 'Playfair Display')!;
  const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
  // Đủ ba vai để kiểm xem trước và khoá.
  const f = (await api.queryFaces({ limit: 200 })).items;
  await api.setComboSlot(res.comboId, 'body', { hashHex: f.find(x => x.family !== 'Playfair Display')!.hashHex, faceIndex: 0 }).catch(() => undefined);
  await api.addComboRole(res.comboId, 'caption'); // vai trống
  render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null }} />); await settle();
  return { api, res };
}

describe('Desk theo ngữ cảnh (Combo / vai)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it('thanh Combo nằm TRÊN danh sách vai và thấy ngay, kèm nút gợi ý (không cần mở menu)', async () => {
    await mount();
    const bar = screen.getByTestId('desk-combobar'); const roles = screen.getAllByTestId('desk-role')[0];
    expect(bar.compareDocumentPosition(roles) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(within(bar).getByTestId('combo-stepper')).toBeTruthy(); expect(within(bar).getByTestId('desk-combo-suggest')).toBeTruthy();
    expect(bar.dataset.selected).toBeUndefined(); // mở Desk: đang chọn một vai
    expect(screen.getByTestId('pick-vi')).toBeTruthy();
  });

  it('xem trước có đủ MỌI vai của Combo (kể cả vai trống) và bấm vai ở xem trước chọn vai đó', async () => {
    const { res } = await mount();
    const listed = screen.getAllByTestId('desk-role').map(r => r.dataset.role);
    const prev = screen.getAllByTestId('desk-prev-role');
    expect(prev.map(p => p.dataset.role)).toEqual(listed);
    expect(prev.length).toBeGreaterThanOrEqual(2);
    expect(prev.some(p => p.dataset.empty === 'true')).toBe(true); // vai chưa có font = ô trống
    void res;
    const heading = prev.find(p => p.dataset.role !== 'body')!;
    fireEvent.click(within(heading).getByTestId('desk-prev-pick')); await settle();
    expect(useUiStore.getState().desk?.role).toBe(heading.dataset.role); // Desk mở lại ở vai vừa bấm (Desk trong test nhận desk cố định nên không tự chuyển)
  });

  it('bấm Combo → cột giữa là gợi ý combo "Gợi ý cho: Combo n", chỉ MỘT trạng thái chọn; bấm lại một vai → về ứng viên của vai', async () => {
    await mount();
    expect(screen.getByRole('heading', { level: 2, name: /Gợi ý cho:/ }).textContent).toMatch(/Gợi ý cho: Body/);
    fireEvent.click(screen.getByTestId('desk-combo-suggest')); await settle();
    expect(screen.getByTestId('desk-combobar').dataset.selected).toBe('true');
    expect(screen.getByTestId('cs-title').textContent).toMatch(/^Gợi ý cho: Combo \d/);
    expect(screen.queryByTestId('pick-vi')).toBeNull(); // cột ứng viên vai đã nhường chỗ
    expect(screen.getAllByTestId('desk-role').every(b => b.getAttribute('aria-pressed') === 'false')).toBe(true);
    expect(screen.getAllByTestId('desk-prev-role').every(p => p.dataset.selected === undefined)).toBe(true);
    fireEvent.click(screen.getAllByTestId('desk-role')[0]); await settle();
    expect(screen.getByTestId('desk-combobar').dataset.selected).toBeUndefined();
    expect(screen.queryByTestId('combo-suggest')).toBeNull();
    expect(screen.getByTestId('pick-vi')).toBeTruthy();
    expect(screen.getAllByTestId('desk-role').filter(b => b.getAttribute('aria-pressed') === 'true')).toHaveLength(1);
  });

  it('mọi vai đã có font: chỉ khoá vai đầu (không còn trả lại đúng combo hiện tại) và hiện ghi chú', async () => {
    const { api } = await mount();
    const spy = vi.spyOn(api, 'suggestCombos');
    fireEvent.click(screen.getByTestId('desk-combo-suggest')); await settle();
    const req = spy.mock.calls.at(-1)![0];
    const filled = screen.getAllByTestId('desk-role').filter(b => !b.querySelector('.fa-desk-role-empty')).length;
    if (filled >= req.roles.length) {
      expect(req.locked.length).toBeLessThanOrEqual(1);
      expect(screen.getByTestId('cs-relaxed')).toBeTruthy();
    }
  });

  it('Combo chỉ có MỘT vai (mở từ một font): gợi ý vẫn có vai đối tác thay vì trả lại đúng font đó', async () => {
    const api = createMockApi();
    const r = (await api.queryFaces({ limit: 200 })).items.find(x => x.family === 'Abril Fatface')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    const spy = vi.spyOn(api, 'suggestCombos');
    render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: res.otherRole, trial: res.trial }} />); await settle();
    fireEvent.click(screen.getByTestId('desk-combo-suggest')); await settle();
    const req = spy.mock.calls.at(-1)![0];
    expect(req.roles.length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByTestId('cs-card').length).toBeGreaterThan(0);
  });
});
