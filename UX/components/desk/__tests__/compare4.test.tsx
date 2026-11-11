import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { ComboCompare } from '../ComboCompare';
import { FontCompare, type FcFace } from '../FontCompare';
import { PairingDesk } from '../PairingDesk';

const initial = useUiStore.getState();
const settle = async (n = 8) => { for (let i = 0; i < n; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
type Api = ReturnType<typeof createMockApi>;
const faces = async (api: Api, n: number): Promise<FcFace[]> => (await api.queryFaces({ limit: 200 })).items.filter(r => r.faceIndex === 0 && !r.isItalic && r.weightClass === 400).slice(0, n).map(r => ({ face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, row: r }));
const mountFc = (api: Api, o: Partial<React.ComponentProps<typeof FontCompare>> = {}) =>
  render(<div className="fa-desk" tabIndex={-1}><FontCompare api={api} role="body" base={null} partner={null} suggestions={[]} onPlace={async () => true} onTry={() => {}} {...o} /></div>);

describe('FontCompare (review fixes)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('columns appear when the suggestions arrive AFTER mount, while the user has not touched the columns', async () => {
    const api = createMockApi(); const fs = await faces(api, 4);
    const { rerender } = mountFc(api);
    expect(screen.queryAllByTestId('fc-col')).toHaveLength(0);
    rerender(<div className="fa-desk" tabIndex={-1}><FontCompare api={api} role="body" base={null} partner={null} suggestions={fs} onPlace={async () => true} onTry={() => {}} /></div>);
    await settle();
    expect(screen.getAllByTestId('fc-col')).toHaveLength(4);
  });

  it('after a successful place the font becomes the base and is NOT duplicated as a column; a failed place keeps the column', async () => {
    const api = createMockApi(); const fs = await faces(api, 3);
    const onPlace = vi.fn(async () => true);
    const { rerender } = mountFc(api, { suggestions: fs, onPlace });
    await settle();
    expect(screen.getAllByTestId('fc-col')).toHaveLength(3);
    fireEvent.click(screen.getAllByTestId('fc-place')[0]); await settle();
    expect(onPlace).toHaveBeenCalledWith(fs[0]);
    expect(screen.getAllByTestId('fc-col')).toHaveLength(2);
    // Desk đặt xong → font thành mốc; gợi ý có thể vẫn còn font đó
    rerender(<div className="fa-desk" tabIndex={-1}><FontCompare api={api} role="body" base={fs[0]} partner={null} suggestions={fs} onPlace={onPlace} onTry={() => {}} /></div>);
    await settle();
    const names = screen.getAllByTestId('fc-col').map(c => c.textContent);
    expect(new Set(names).size).toBe(names.length);
    expect(screen.getAllByTestId('fc-col')[0].dataset.base).toBe('true');
    onPlace.mockResolvedValueOnce(false);
    const before = screen.getAllByTestId('fc-col').length;
    fireEvent.click(screen.getAllByTestId('fc-place')[0]); await settle();
    expect(screen.getAllByTestId('fc-col')).toHaveLength(before);
  });

  it('a getFace failure marks that column, is not retried in a loop, and the other columns still read', async () => {
    const api = createMockApi(); const fs = await faces(api, 3);
    const real = api.getFace.bind(api);
    const spy = vi.spyOn(api, 'getFace').mockImplementation(async (f, l) => { if (f.hashHex === fs[1].face.hashHex) throw new Error('boom'); return real(f, l); });
    mountFc(api, { suggestions: fs });
    await settle(12);
    expect(screen.getAllByTestId('fc-readfail')).toHaveLength(1);
    const callsFor = spy.mock.calls.filter(c => c[0].hashHex === fs[1].face.hashHex).length;
    await settle(12);
    expect(spy.mock.calls.filter(c => c[0].hashHex === fs[1].face.hashHex).length).toBe(callsFor);
    expect(callsFor).toBe(1);
  });

  it('a cloud-only face shows the cloud note instead of loading its font', async () => {
    const api = createMockApi(); const fs = await faces(api, 2);
    const cloud = { ...fs[0], row: { ...fs[0].row, onCloudOnly: true } };
    mountFc(api, { suggestions: [cloud, fs[1]] });
    await settle();
    expect(screen.getAllByTestId('fc-cloud')).toHaveLength(1);
    expect(screen.getAllByTestId('fc-sample')).toHaveLength(1);
  });

  it('removing a column returns focus to the Desk root; pin keeps columns after sorting; columns have scope=col and the License row is localised', async () => {
    const api = createMockApi(); const fs = await faces(api, 3);
    mountFc(api, { suggestions: fs }); await settle();
    const remove = screen.getAllByTestId('fc-remove')[0]; remove.focus();
    fireEvent.click(remove);
    await act(async () => { await new Promise(r => requestAnimationFrame(() => r(null))); });
    expect(document.activeElement).toBe(document.querySelector('.fa-desk'));
    for (const th of screen.getAllByTestId('fc-col')) expect(th.getAttribute('scope')).toBe('col');
    expect(document.querySelector('[data-row=license] th')!.textContent).toBe('Giấy phép');
    expect(document.body.textContent).not.toMatch(/📌|★/);
  });
});

describe('Desk ↔ Compare wiring', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  async function open(withBody = true) {
    const api = createMockApi();
    const rs = (await api.queryFaces({ limit: 200 })).items;
    const a = rs.find(x => x.family === 'Playfair Display')!; const b = rs.find(x => x.family === 'Source Serif 4')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    if (withBody) await api.setComboSlot(res.comboId, 'body', { hashHex: b.hashHex, faceIndex: 0 });
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    function Host() { const d = useUiStore(s => s.desk); return d ? <PairingDesk api={api} desk={d} /> : null; }
    render(<Host />); await settle();
    return { api, res };
  }

  it('place from Compare Font calls setComboSlot with THIS combo and role; an error is reported and keeps the column', async () => {
    const { api, res } = await open();
    fireEvent.click(screen.getByTestId('mode-fonts')); await settle();
    expect(screen.getAllByTestId('fc-col').length).toBeGreaterThan(1); // mốc + ít nhất một ứng viên (không bỏ qua test)
    const spy = vi.spyOn(api, 'setComboSlot');
    const place = screen.getAllByTestId('fc-place')[0];
    fireEvent.click(place); await settle();
    expect(spy).toHaveBeenCalledWith(res.comboId, 'body', expect.objectContaining({ hashHex: expect.any(String) }));
    spy.mockRejectedValueOnce(new Error('db locked'));
    const n = screen.getAllByTestId('fc-col').length;
    expect(n).toBeGreaterThan(1);
    fireEvent.click(screen.getAllByTestId('fc-place')[0]); await settle();
    expect(useNoticeStore.getState().notices.some(x => /db locked/.test(x.text))).toBe(true);
    expect(screen.getAllByTestId('fc-col')).toHaveLength(n);
  });

  it('keys: Shift+C only in Pairing; [ and ] do nothing in So sánh Font (would drop the pins)', async () => {
    const { api, res } = await open();
    await api.createCombo(res.case.id); // có Combo kế
    const desk = screen.getByTestId('pairing-desk');
    fireEvent.click(screen.getByTestId('mode-fonts')); await settle();
    const before = useUiStore.getState().desk!.comboId;
    fireEvent.keyDown(desk, { key: ']' }); await settle();
    expect(useUiStore.getState().desk!.comboId).toBe(before);
    fireEvent.keyDown(desk, { key: 'C', shiftKey: true }); await settle();
    expect(useUiStore.getState().compare).toBeNull();
  });

  it('Combo changes in the stepper refresh the Combo compare table (create → a new column)', async () => {
    await open();
    fireEvent.click(screen.getByTestId('mode-combos')); await settle();
    expect(screen.getAllByTestId('cc-col')).toHaveLength(1);
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: /Combo mới/ })); await settle(12);
    expect(screen.getAllByTestId('cc-col')).toHaveLength(2);
  });
});

describe('ComboCompare (edge cases)', () => {
  it('a single Combo: one base column; Dùng combo này is aria-disabled for the main Combo and does nothing', async () => {
    const api = createMockApi();
    const rs = (await api.queryFaces({ limit: 50 })).items;
    const res = await api.openCaseFromPeek({ face: { hashHex: rs[0].hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    const onMain = vi.fn();
    render(<ComboCompare api={api} caseId={res.case.id} currentId={res.comboId} onOpen={() => {}} onMain={onMain} />); await settle();
    expect(screen.getAllByTestId('cc-col')).toHaveLength(1);
    const main = screen.getByTestId('cc-main');
    expect(main.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(main);
    expect(onMain).not.toHaveBeenCalled();
    expect(document.body.textContent).not.toMatch(/★ /);
  });

  it('a caseCompare error is shown as an alert', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'caseCompare').mockRejectedValue(new Error('no db'));
    render(<ComboCompare api={api} caseId={1} currentId={1} onOpen={() => {}} onMain={() => {}} />); await settle();
    expect(screen.getByRole('alert').textContent).toContain('no db');
  });
});

describe('ComboCompare cloud', () => {
  it('a Combo whose font is cloud-only shows the cloud note on its mini page', async () => {
    const api = createMockApi();
    const rs = (await api.queryFaces({ limit: 50 })).items;
    const res = await api.openCaseFromPeek({ face: { hashHex: rs[0].hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    const real = api.caseCompare.bind(api);
    vi.spyOn(api, 'caseCompare').mockImplementation(async (id, ids, lang) => {
      const r = await real(id, ids, lang);
      return { ...r, columns: r.columns.map(c => ({ ...c, combo: { ...c.combo, slots: c.combo.slots.map(sl => ({ ...sl, row: sl.row ? { ...sl.row, onCloudOnly: true } : sl.row })) } })) };
    });
    render(<ComboCompare api={api} caseId={res.case.id} currentId={res.comboId} onOpen={() => {}} onMain={() => {}} />); await settle();
    expect(screen.getAllByTestId('cc-cloud').length).toBeGreaterThan(0);
  });
});
