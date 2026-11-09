import { describe, expect, it } from 'vitest';
import { CmdError } from '../types';
import { createMockApi, mockRows } from '../mock';

const key = (r: { hashHex: string; faceIndex: number }) => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });

describe('mock Case/Combo api (mirrors backend user.db v6)', () => {
  it('addToCase without case creates one, then reuses the most recently opened', async () => {
    const api = createMockApi();
    const [a, b] = mockRows().slice(0, 2).map(key);
    const r1 = await api.addToCase(null, [a]);
    expect(r1).toMatchObject({ created: true, added: 1, alreadyIn: 0 });
    const r2 = await api.addToCase(null, [a, b], 'pinned');
    expect(r2).toMatchObject({ caseId: r1.caseId, created: false, added: 1, alreadyIn: 1 });
    const c = await api.getCase(r1.caseId);
    expect(c.name).toBe('Case 1');
    expect(c.faces.map(f => f.origin)).toEqual(['added', 'pinned']);
    expect(c.faces.every(f => f.row)).toBe(true);
  });

  it('combos: first is main, numbering stays contiguous, slots add faces to the case', async () => {
    const api = createMockApi();
    const [a, b, c3] = mockRows().slice(0, 3).map(key);
    const c = await api.createCase('Poster');
    const m1 = await api.createCombo(c.id);
    const m2 = await api.createCombo(c.id, 'Lạnh');
    const m3 = await api.createCombo(c.id, undefined, m1.id);
    expect([m1.isMain, m2.isMain, m3.idx]).toEqual([true, false, 3]);
    await expect(api.setComboSlot(m1.id, 'nope' as never, a)).rejects.toMatchObject({ code: 'invalid_arg' });
    const withSlots = await api.setComboSlot(m1.id, 'body', b);
    const heading = await api.setComboSlot(withSlots.id, 'heading', a, 'neo', { weight: 700 });
    expect(heading.slots.map(s => s.role)).toEqual(['heading', 'body']);
    expect((await api.getCase(c.id)).faces).toHaveLength(2);
    expect((await api.setComboSlot(m1.id, 'heading', c3)).slots.find(s => s.role === 'heading')).toMatchObject({ note: 'neo', style: { weight: 700 } });
    expect((await api.setComboSlot(m1.id, 'body', null)).slots).toHaveLength(1);

    await api.updateCombo(m2.id, { isMain: true });
    expect((await api.getCase(c.id)).combos.map(x => x.isMain)).toEqual([false, true, false]);
    await api.reorderCombos(c.id, [m3.id, m2.id, m1.id]);
    expect((await api.getCase(c.id)).combos.map(x => [x.id, x.idx])).toEqual([[m3.id, 1], [m2.id, 2], [m1.id, 3]]);
    await expect(api.reorderCombos(c.id, [m1.id])).rejects.toBeInstanceOf(CmdError);
    await api.deleteCombo(m2.id);
    const left = (await api.getCase(c.id)).combos;
    expect(left.map(x => x.idx)).toEqual([1, 2]);
    expect(left.filter(x => x.isMain)).toHaveLength(1);
  });

  it('removeFromCase drops the face from combos; createCombos is atomic', async () => {
    const api = createMockApi();
    const [a, b] = mockRows().slice(0, 2).map(key);
    const c = await api.createCase();
    await expect(api.createCombos(c.id, [{ name: 'ok', slots: [{ role: 'body', face: a }] }, { name: null, slots: [{ role: 'x' as never, face: b }] }])).rejects.toBeInstanceOf(CmdError);
    expect(await api.getCase(c.id)).toMatchObject({ faces: [], combos: [] });
    const [m] = await api.createCombos(c.id, [{ name: null, slots: [{ role: 'body', face: a }, { role: 'heading', face: b }] }]);
    expect(m.isMain).toBe(true);
    expect(await api.removeFromCase(c.id, [b])).toBe(1);
    expect((await api.getCase(c.id)).combos[0].slots.map(s => s.role)).toEqual(['body']);
  });

  it('soft delete hides the case, restore brings it back', async () => {
    const api = createMockApi();
    const c = await api.createCase('A');
    await api.deleteCase(c.id);
    expect(await api.listCases()).toHaveLength(0);
    expect(await api.listCases(true)).toHaveLength(1);
    await expect(api.getCase(c.id)).rejects.toMatchObject({ code: 'not_found' });
    expect((await api.restoreCase(c.id)).deletedAt).toBeNull();
    await expect(api.createCase('  ')).rejects.toMatchObject({ code: 'invalid_arg' });
  });

  it('caseCompare returns one column per combo; openCaseFromPeek anchors the font and returns the candidate as trial', async () => {
    const api = createMockApi();
    const [a, b] = mockRows().slice(0, 2).map(key);
    const r = await api.openCaseFromPeek({ face: a, as: 'heading', candidate: b, caseId: null, settings: { vi: false, mode: 'safe' } });
    expect(r).toMatchObject({ created: true, anchorRole: 'display', otherRole: 'body', trial: b });
    expect(r.case.settings).toEqual({ vi: false, mode: 'safe' });
    expect(r.case.combos[0].slots.map(s => s.role)).toEqual(['display']);
    expect(r.case.faces.map(f => f.origin)).toEqual(['peek', 'peek']);
    const r2 = await api.openCaseFromPeek({ face: b, as: 'body', candidate: null, caseId: null, settings: { vi: true, mode: null } });
    expect(r2).toMatchObject({ created: false, comboId: r.comboId, anchorRole: 'body', otherRole: 'display', trial: null });
    const cmp = await api.caseCompare(r.case.id, []);
    expect(cmp.columns).toHaveLength(1);
    expect(cmp.columns[0]).toMatchObject({ fontCount: 2, relations: [] });
    await expect(api.caseCompare(r.case.id, [999])).rejects.toMatchObject({ code: 'not_found' });
    await expect(api.openCaseFromPeek({ face: a, as: 'body', candidate: null, caseId: 999, settings: { vi: true, mode: null } })).rejects.toMatchObject({ code: 'not_found' });
  });
});
