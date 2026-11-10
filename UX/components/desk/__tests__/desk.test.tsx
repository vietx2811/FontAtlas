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
    expect(screen.getByTestId('desk-place')).not.toBeDisabled();
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
