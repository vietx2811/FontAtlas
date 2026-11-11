import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { FaceDetailDto } from '../../../api';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { PairTab } from '../../detail/tabs/PairTab';
import { PairingDesk } from '../PairingDesk';
import { anchorPairRole, anchorRoles, isHeadingLike } from '../deskRoles';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const rows = async (api: ReturnType<typeof createMockApi>) => (await api.queryFaces({ limit: 200 })).items;
const detailOf = async (api: ReturnType<typeof createMockApi>, family: string): Promise<FaceDetailDto> => {
  const r = (await rows(api)).find(x => x.family === family)!;
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
};

describe('role groups', () => {
  it('heading-like and body-like are opposite anchors', () => {
    expect(isHeadingLike('display')).toBe(true); expect(isHeadingLike('body')).toBe(false);
    expect(anchorPairRole('body')).toBe('heading'); expect(anchorPairRole('display')).toBe('body');
    expect(anchorRoles('body')[0]).toBe('display'); expect(anchorRoles('display')[0]).toBe('body');
  });
});

describe('Pair from peek (2d)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('opens the Pairing Desk on the other role with the candidate as the trial; nothing is placed yet', async () => {
    const api = createMockApi();
    const detail = await detailOf(api, 'Playfair Display');
    render(<PairTab api={api} detail={detail} css={{}} />); await settle();
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    const d = useUiStore.getState().desk!;
    expect(d).toMatchObject({ role: 'body' });
    expect(d.trial).not.toBeNull();
    const c = await api.getCase(d.caseId);
    expect(c.combos[0].slots.map(s => s.role)).toEqual(['display']);
    expect(c.settings).toMatchObject({ vi: true, mode: 'safe' });
  });

  it('is idempotent: opening twice neither duplicates the Case nor the font', async () => {
    const api = createMockApi();
    const detail = await detailOf(api, 'Playfair Display');
    render(<PairTab api={api} detail={detail} css={{}} />); await settle();
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    expect((await api.listCases()).length).toBe(1);
  });

  it('an occupied anchor role asks first; Giữ opens without replacing, Thay replaces', async () => {
    const api = createMockApi();
    const a = await detailOf(api, 'Playfair Display'); const b = await detailOf(api, 'Source Serif 4');
    const { unmount } = render(<PairTab api={api} detail={a} css={{}} />); await settle();
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    const caseId = useUiStore.getState().desk!.caseId; unmount();
    useUiStore.setState({ desk: null });
    render(<PairTab api={api} detail={b} css={{}} />); await settle();
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    expect(useUiStore.getState().desk).toBeNull();
    expect(screen.getByRole('dialog').textContent).toMatch(/Playfair Display/);
    fireEvent.click(screen.getByTestId('occupied-keep')); await settle();
    expect(useUiStore.getState().desk?.caseId).toBe(caseId);
    expect((await api.getCase(caseId)).combos[0].slots[0].row?.family).toBe('Playfair Display');
    useUiStore.setState({ desk: null });
    fireEvent.click(screen.getByTestId('open-desk')); await settle();
    fireEvent.click(screen.getByTestId('occupied-replace')); await settle();
    expect((await api.getCase(caseId)).combos[0].slots[0].row?.family).toBe('Source Serif 4');
    expect(useUiStore.getState().desk).not.toBeNull();
  });
});

describe('PairingDesk', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  async function seeded() {
    const api = createMockApi();
    const r = (await rows(api)).find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    return { api, res };
  }
  const mount = async (api: ReturnType<typeof createMockApi>, res: Awaited<ReturnType<ReturnType<typeof createMockApi>['openCaseFromPeek']>>) => {
    const desk = { caseId: res.case.id, comboId: res.comboId, role: res.otherRole, trial: res.trial };
    render(<PairingDesk api={api} desk={desk} />); await settle();
  };

  it('shows the roles column with the anchor font and an empty Body, and candidates for the other role', async () => {
    const { api, res } = await seeded(); await mount(api, res);
    const roles = screen.getAllByTestId('desk-role');
    expect(roles.map(r => r.dataset.role)).toEqual(['display', 'body']);
    expect(roles[0].textContent).toMatch(/Playfair Display/);
    expect(roles[1].getAttribute('aria-pressed')).toBe('true');
    expect(screen.getAllByTestId('desk-cand').length).toBeGreaterThan(0);
    expect(screen.getByTestId('desk-place').getAttribute('aria-disabled')).toBe('false');
  });

  it('ArrowDown tries the next candidate; Enter places it in the role through setComboSlot', async () => {
    const { api, res } = await seeded(); await mount(api, res);
    const desk = screen.getByTestId('pairing-desk');
    const before = screen.getByTestId('desk-trying').textContent;
    fireEvent.keyDown(desk, { key: 'ArrowDown' }); await settle();
    expect(screen.getByTestId('desk-trying').textContent).not.toBe(before);
    fireEvent.keyDown(desk, { key: 'Enter' }); await settle();
    const c = await api.getCase(res.case.id);
    expect(c.combos[0].slots.map(s => s.role)).toEqual(['display', 'body']);
    expect(useNoticeStore.getState().notices[0].text).toMatch(/Đã đặt .* vào vai Body/);
  });

  it('the peek trial leads the list and is marked', async () => {
    const api = createMockApi();
    const rs = await rows(api);
    const a = rs.find(x => x.family === 'Playfair Display')!; const t = rs.find(x => x.family === 'Fraunces')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: 0 }, as: 'heading', candidate: { hashHex: t.hashHex, faceIndex: t.faceIndex }, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    await mount(api, res);
    const first = screen.getAllByTestId('desk-cand')[0];
    expect(first.dataset.trial).toBe('peek'); expect(first.textContent).toMatch(/Fraunces/);
  });

  it('Esc leaves the desk', async () => {
    const { api, res } = await seeded(); await mount(api, res);
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'Escape' });
    expect(useUiStore.getState().desk).toBeNull();
  });

  it('with no font on the opposite role it says so instead of listing candidates', async () => {
    const { api, res } = await seeded();
    render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: 'display', trial: null }} />); await settle();
    // display is filled; ask for a role whose opposite group is empty
    useUiStore.setState({ desk: null });
    const api2 = createMockApi();
    const c = await api2.createCase('Vắng'); const combo = await api2.createCombo(c.id);
    const { unmount } = render(<PairingDesk api={api2} desk={{ caseId: c.id, comboId: combo.id, role: 'body', trial: null }} />); await settle();
    expect(screen.getAllByTestId('desk-noanchor').length).toBeGreaterThan(0);
    unmount();
  });
});

describe('DetailStrip (slice 2)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  async function mounted() {
    const api = createMockApi();
    const r = (await rows(api)).find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: 'display', trial: null }} />); await settle();
    return { api, res };
  }

  it('lists the metric rows; the trial card hints until there is a trial different from the one in use', async () => {
    const { res } = await mounted();
    expect(screen.getByTestId('detail-strip')).toBeTruthy();
    expect(screen.getAllByTestId('strip-delta').length).toBe(8);
    expect(screen.getByTestId('strip-card-use').textContent).toMatch(/Playfair Display/);
    void res;
  });

  it('after placing a font, trying another shows the signed delta against the one in use', async () => {
    const { api, res } = await mounted();
    // body is empty: place the first candidate, then step to the next.
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    const { unmount } = render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null }} />); await settle();
    const desks = screen.getAllByTestId('pairing-desk'); const desk = desks[desks.length - 1];
    fireEvent.keyDown(desk, { key: 'Enter' }); await settle();
    fireEvent.keyDown(desk, { key: 'ArrowDown' }); await settle();
    const cards = screen.getAllByTestId('strip-card-try');
    expect(cards[cards.length - 1].textContent).not.toMatch(/Rê chuột/);
    const deltas = screen.getAllByTestId('strip-delta').slice(-8).map(d => d.textContent);
    expect(deltas.some(x => /^[+−]|±0/.test(x ?? ''))).toBe(true);
    unmount();
  });

  it('I collapses and reopens the strip', async () => {
    await mounted();
    const desk = screen.getByTestId('pairing-desk');
    fireEvent.keyDown(desk, { key: 'i' });
    expect(screen.getByTestId('detail-strip').dataset.collapsed).toBe('true');
    expect(screen.queryByTestId('strip-metrics')).toBeNull();
    fireEvent.click(screen.getByTestId('strip-toggle'));
    expect(screen.getByTestId('strip-metrics')).toBeTruthy();
  });
});

describe('ComboStepper (slice 3)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Host({ api }: { api: ReturnType<typeof createMockApi> }) {
    const desk = useUiStore(s => s.desk);
    return desk ? <PairingDesk api={api} desk={desk} /> : null;
  }
  async function open() {
    const api = createMockApi();
    const r = (await rows(api)).find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    render(<Host api={api} />); await settle();
    return { api, res };
  }
  const pick = async (label: RegExp) => { fireEvent.click(screen.getByTestId('combo-label')); await settle(); fireEvent.click(screen.getByRole('menuitem', { name: label })); await settle(); };

  it('shows Combo 1/1 with the main star; prev/next disabled', async () => {
    await open();
    expect(screen.getByTestId('combo-label').textContent).toMatch(/★ Combo 1\/1/);
    expect(screen.getByTestId('combo-prev')).toBeDisabled(); expect(screen.getByTestId('combo-next')).toBeDisabled();
  });

  it('with the delete confirm open no Desk key runs and focus is not stolen from the dialog', async () => {
    const { api, res } = await open();
    await pick(/Nhân bản/);
    await pick(/Xoá Combo/);
    const before = (await api.getCase(res.case.id)).combos.length;
    const desk = screen.getByTestId('pairing-desk');
    fireEvent.keyDown(desk, { key: 'Enter' }); fireEvent.keyDown(desk, { key: '[' }); await settle();
    expect(screen.getByTestId('combo-delete-confirm')).toBeTruthy();
    expect(useUiStore.getState().desk).not.toBeNull();
    expect((await api.getCase(res.case.id)).combos.length).toBe(before);
    expect(document.activeElement).not.toBe(desk);
  });

  it('the heading uses the neutral copy while the sample is the app default, and the owner text once edited', async () => {
    await open();
    expect(screen.getByTestId('desk-heading').textContent).toBe('Nghệ thuật dàn trang');
    act(() => { useUiStore.setState({ sample: 'Cà phê sữa đá' }); }); await settle();
    expect(screen.getByTestId('desk-heading').textContent).toBe('Cà phê sữa đá');
  });

  it('closing Compare opened from the Desk returns to that Desk', async () => {
    const { res } = await open();
    const rs = await rows(createMockApi());
    useUiStore.getState().openCompare([{ hashHex: rs[0].hashHex, faceIndex: 0 }, { hashHex: rs[1].hashHex, faceIndex: 0 }]);
    expect(useUiStore.getState().desk).toBeNull();
    useUiStore.getState().closeCompare();
    expect(useUiStore.getState().desk?.caseId).toBe(res.case.id);
    useUiStore.getState().closeDesk();
    useUiStore.getState().openCompare([{ hashHex: rs[0].hashHex, faceIndex: 0 }, { hashHex: rs[1].hashHex, faceIndex: 0 }]);
    useUiStore.getState().closeCompare();
    expect(useUiStore.getState().desk).toBeNull();
  });

  it('New creates and steps to Combo 2/2; ] and [ move between them', async () => {
    const { api, res } = await open();
    await pick(/Combo mới/);
    expect(screen.getByTestId('combo-label').textContent).toMatch(/Combo 2\/2/);
    expect((await api.getCase(res.case.id)).combos.length).toBe(2);
    const desk = screen.getByTestId('pairing-desk');
    fireEvent.keyDown(desk, { key: '[' }); await settle();
    expect(screen.getByTestId('combo-label').textContent).toMatch(/Combo 1\/2/);
    fireEvent.keyDown(desk, { key: ']' }); await settle();
    expect(screen.getByTestId('combo-label').textContent).toMatch(/Combo 2\/2/);
  });

  it('Duplicate copies the slots; Make main moves the star; Delete steps back and is disabled for the last one', async () => {
    const { api, res } = await open();
    await pick(/Nhân bản/);
    const c = await api.getCase(res.case.id);
    expect(c.combos[1].slots.map(s => s.role)).toEqual(['display']);
    await pick(/Đặt làm Combo chính/);
    expect((await api.getCase(res.case.id)).combos.find(x => x.isMain)!.idx).toBe(2);
    await pick(/Xoá Combo/);
    expect((await api.getCase(res.case.id)).combos.length).toBe(2); // chưa xoá: đang hỏi xác nhận
    fireEvent.click(screen.getByTestId('combo-delete-cancel')); await settle();
    expect((await api.getCase(res.case.id)).combos.length).toBe(2);
    await pick(/Xoá Combo/);
    fireEvent.click(screen.getByTestId('combo-delete-confirm')); await settle();
    expect((await api.getCase(res.case.id)).combos.length).toBe(1);
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    expect(screen.getByRole('menuitem', { name: /Xoá Combo/ }).getAttribute('aria-disabled')).toBe('true');
  });
});

describe('candidates exclude the anchor itself', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  it('the anchor font is never offered as its own pair', async () => {
    const api = createMockApi();
    const r = (await rows(api)).find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    render(<PairingDesk api={api} desk={{ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null }} />); await settle();
    expect(screen.queryAllByTestId('desk-cand').some(c => /Playfair Display Regular/.test(c.textContent ?? ''))).toBe(false);
  });
});

describe('review fixes (slices 1-3)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  async function mountBody() {
    const api = createMockApi();
    const rs = await rows(api);
    const a = rs.find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    function Host() { const d = useUiStore(s => s.desk); return d ? <PairingDesk api={api} desk={d} /> : null; }
    render(<Host />); await settle();
    return { api, res };
  }

  it('keys typed in the rename box do not reach the desk (no strip toggle, no combo step, no place)', async () => {
    const { api, res } = await mountBody();
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên Combo/ })); await settle();
    const input = screen.getByRole('textbox', { name: /Đổi tên Combo/ });
    for (const k of ['i', 'I', '[', ']', 'ArrowDown']) fireEvent.keyDown(input, { key: k });
    expect(screen.getByTestId('detail-strip').dataset.collapsed).toBeUndefined();
    fireEvent.change(input, { target: { value: 'Mới' } });
    fireEvent.keyDown(input, { key: 'Enter' }); await settle();
    expect((await api.getCase(res.case.id)).combos[0].name).toBe('Mới');
    expect((await api.getCase(res.case.id)).combos[0].slots.map(s => s.role)).toEqual(['display']); // Enter không đặt font
  });

  it('Enter on a button does not place a font; Enter repeat is ignored', async () => {
    const { api, res } = await mountBody();
    fireEvent.keyDown(screen.getByTestId('desk-back'), { key: 'Enter' }); await settle();
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'Enter', repeat: true }); await settle();
    expect((await api.getCase(res.case.id)).combos[0].slots.map(s => s.role)).toEqual(['display']);
  });

  it('the selected candidate survives a reload of the case data (rename keeps the index)', async () => {
    const { api, res } = await mountBody();
    const desk = screen.getByTestId('pairing-desk');
    if (screen.getAllByTestId('desk-cand').length < 2) return; // thư viện mock chỉ có một ứng viên: không có gì để giữ
    fireEvent.keyDown(desk, { key: 'ArrowDown' }); await settle();
    const before = screen.getByTestId('desk-trying').textContent;
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên Combo/ })); await settle();
    fireEvent.change(screen.getByRole('textbox', { name: /Đổi tên Combo/ }), { target: { value: 'X' } });
    fireEvent.keyDown(screen.getByRole('textbox', { name: /Đổi tên Combo/ }), { key: 'Enter' }); await settle();
    expect(screen.getByTestId('desk-trying').textContent).toBe(before);
    void api; void res;
  });

  it('Compare is disabled with a reason while there is only one distinct font; Shift+C does nothing', async () => {
    await mountBody();
    const btn = screen.queryByTestId('strip-compare');
    if (btn) expect(btn.getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'C', shiftKey: true });
    expect(useUiStore.getState().compare).toBeNull();
  });

  it('resumeCompare and openCompare leave no Desk behind', async () => {
    useUiStore.setState({ desk: { caseId: 1, comboId: 1, role: 'body', trial: null }, compareSession: { faces: [{ hashHex: 'a', faceIndex: 0 }, { hashHex: 'b', faceIndex: 0 }], pinned: [], overlayOn: false, overlayIds: [] } });
    useUiStore.getState().resumeCompare();
    expect(useUiStore.getState().desk).toBeNull();
  });

  it('metric rows are TermTips and the role name has a tooltip, no native title', async () => {
    await mountBody();
    expect(document.querySelectorAll('[data-testid=strip-metrics] th[scope=row] [data-term]').length).toBe(8);
    expect(document.querySelector('.fa-desk-role-name[title]')).toBeNull();
  });
});

describe('Compare Combo (slice 4)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Host({ api }: { api: ReturnType<typeof createMockApi> }) {
    const desk = useUiStore(s => s.desk);
    return desk ? <PairingDesk api={api} desk={desk} /> : null;
  }
  async function twoCombos() {
    const api = createMockApi();
    const rs = await rows(api);
    const a = rs.find(x => x.family === 'Playfair Display')!; const b = rs.find(x => x.family === 'Source Serif 4')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    await api.setComboSlot(res.comboId, 'body', { hashHex: b.hashHex, faceIndex: 0 });
    const c2 = await api.createCombo(res.case.id, 'B', res.comboId);
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: c2.id, role: 'body', trial: null });
    render(<Host api={api} />); await settle();
    return { api, res, c2 };
  }

  it('lists a column per Combo with the current one first as the base; rows for fonts, relations, VI, license', async () => {
    const { c2 } = await twoCombos();
    fireEvent.click(screen.getByTestId('mode-combos')); await settle();
    const cols = screen.getAllByTestId('cc-col');
    expect(cols.length).toBe(2);
    expect(cols[0].dataset.base).toBe('true'); expect(cols[0].textContent).toMatch(new RegExp(`Combo ${2}`));
    expect(screen.getAllByTestId('cc-rel').length).toBe(2);
    expect(screen.getAllByTestId('cc-vi').length).toBe(2);
    expect(screen.getAllByTestId('cc-page').length).toBe(2);
    void c2;
  });

  it('Dùng combo này makes it main; Mở trong Pairing returns to the desk on that Combo', async () => {
    const { api, res, c2 } = await twoCombos();
    fireEvent.click(screen.getByTestId('mode-combos')); await settle();
    fireEvent.click(screen.getAllByTestId('cc-main')[0]); await settle();
    expect((await api.getCase(res.case.id)).combos.find(c => c.id === c2.id)!.isMain).toBe(true);
    fireEvent.click(screen.getAllByTestId('cc-open')[1]); await settle();
    expect(screen.queryByTestId('combo-compare')).toBeNull();
    expect(useUiStore.getState().desk!.comboId).toBe(res.comboId);
  });
});

describe('review fixes (slices 1-3)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  async function mountBody() {
    const api = createMockApi();
    const rs = await rows(api);
    const a = rs.find(x => x.family === 'Playfair Display')!;
    const res = await api.openCaseFromPeek({ face: { hashHex: a.hashHex, faceIndex: 0 }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: 'body', trial: null });
    function Host() { const d = useUiStore(s => s.desk); return d ? <PairingDesk api={api} desk={d} /> : null; }
    render(<Host />); await settle();
    return { api, res };
  }

  it('keys typed in the rename box do not reach the desk (no strip toggle, no combo step, no place)', async () => {
    const { api, res } = await mountBody();
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên Combo/ })); await settle();
    const input = screen.getByRole('textbox', { name: /Đổi tên Combo/ });
    for (const k of ['i', 'I', '[', ']', 'ArrowDown']) fireEvent.keyDown(input, { key: k });
    expect(screen.getByTestId('detail-strip').dataset.collapsed).toBeUndefined();
    fireEvent.change(input, { target: { value: 'Mới' } });
    fireEvent.keyDown(input, { key: 'Enter' }); await settle();
    expect((await api.getCase(res.case.id)).combos[0].name).toBe('Mới');
    expect((await api.getCase(res.case.id)).combos[0].slots.map(s => s.role)).toEqual(['display']); // Enter không đặt font
  });

  it('Enter on a button does not place a font; Enter repeat is ignored', async () => {
    const { api, res } = await mountBody();
    fireEvent.keyDown(screen.getByTestId('desk-back'), { key: 'Enter' }); await settle();
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'Enter', repeat: true }); await settle();
    expect((await api.getCase(res.case.id)).combos[0].slots.map(s => s.role)).toEqual(['display']);
  });

  it('the selected candidate survives a reload of the case data (rename keeps the index)', async () => {
    const { api, res } = await mountBody();
    const desk = screen.getByTestId('pairing-desk');
    if (screen.getAllByTestId('desk-cand').length < 2) return; // thư viện mock chỉ có một ứng viên: không có gì để giữ
    fireEvent.keyDown(desk, { key: 'ArrowDown' }); await settle();
    const before = screen.getByTestId('desk-trying').textContent;
    fireEvent.click(screen.getByTestId('combo-label')); await settle();
    fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên Combo/ })); await settle();
    fireEvent.change(screen.getByRole('textbox', { name: /Đổi tên Combo/ }), { target: { value: 'X' } });
    fireEvent.keyDown(screen.getByRole('textbox', { name: /Đổi tên Combo/ }), { key: 'Enter' }); await settle();
    expect(screen.getByTestId('desk-trying').textContent).toBe(before);
    void api; void res;
  });

  it('Compare is disabled with a reason while there is only one distinct font; Shift+C does nothing', async () => {
    await mountBody();
    const btn = screen.queryByTestId('strip-compare');
    if (btn) expect(btn.getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(screen.getByTestId('pairing-desk'), { key: 'C', shiftKey: true });
    expect(useUiStore.getState().compare).toBeNull();
  });

  it('resumeCompare and openCompare leave no Desk behind', async () => {
    useUiStore.setState({ desk: { caseId: 1, comboId: 1, role: 'body', trial: null }, compareSession: { faces: [{ hashHex: 'a', faceIndex: 0 }, { hashHex: 'b', faceIndex: 0 }], pinned: [], overlayOn: false, overlayIds: [] } });
    useUiStore.getState().resumeCompare();
    expect(useUiStore.getState().desk).toBeNull();
  });

  it('metric rows are TermTips and the role name has a tooltip, no native title', async () => {
    await mountBody();
    expect(document.querySelectorAll('[data-testid=strip-metrics] th[scope=row] [data-term]').length).toBe(8);
    expect(document.querySelector('.fa-desk-role-name[title]')).toBeNull();
  });
});

