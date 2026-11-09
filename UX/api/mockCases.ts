// Mock in-memory của Case + Combo (cùng hành vi với backend user.db v6): xoá mềm, Case mặc định = vừa mở,
// Combo đầu là chính, đánh số lại liên tục, đặt font vào vai tự đưa font vào Case.
import { CmdError } from './types';
import type { AddToCaseDto, Api, CaseDto, CaseOriginDto, CaseSummaryDto, ComboDto, ComboRoleDto } from './index';
import type { FaceKeyDto, FaceRowDto } from '../bindings';

const ROLES: ComboRoleDto[] = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote'];
const MAX_FACES = 500, MAX_COMBOS = 50;
const kOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;

type Slot = { role: ComboRoleDto; key: FaceKeyDto; note: string | null; style: unknown };
type Combo = { id: number; caseId: number; idx: number; name: string; isMain: boolean; note: string | null; slots: Slot[] };
type Case = Omit<CaseDto, 'faces' | 'combos'> & { faces: Map<string, { key: FaceKeyDto; origin: CaseOriginDto; note: string | null; addedAt: number }>; combos: Combo[] };

export function createMockCases(rowOf: (k: FaceKeyDto) => FaceRowDto | undefined): Pick<Api,
  'createCase' | 'listCases' | 'openCase' | 'getCase' | 'updateCase' | 'deleteCase' | 'restoreCase' | 'addToCase' | 'removeFromCase' |
  'createCombo' | 'updateCombo' | 'deleteCombo' | 'reorderCombos' | 'setComboSlot' | 'createCombos'> {
  const cases = new Map<number, Case>();
  let seq = 0, comboSeq = 0, tick = 0;
  const now = () => 1_700_000_000_000 + ++tick;
  const live = (id: number) => { const c = cases.get(id); if (!c || c.deletedAt != null) throw new CmdError('not_found', `Case #${id}`); return c; };
  const comboOf = (id: number) => { for (const c of cases.values()) { const b = c.combos.find(x => x.id === id); if (b) return { c, b }; } throw new CmdError('not_found', `Combo #${id}`); };
  const invalid = (m: string) => new CmdError('invalid_arg', m);
  const slotDto = (s: Slot) => ({ role: s.role, key: { ...s.key }, row: rowOf(s.key) ?? null, note: s.note, style: s.style });
  const comboDto = (b: Combo): ComboDto => ({ id: b.id, caseId: b.caseId, idx: b.idx, name: b.name, isMain: b.isMain, note: b.note, slots: [...b.slots].sort((x, y) => ROLES.indexOf(x.role) - ROLES.indexOf(y.role)).map(slotDto) });
  const dto = (c: Case): CaseDto => ({ ...c, faces: [...c.faces.values()].map(f => ({ key: { ...f.key }, origin: f.origin, note: f.note, addedAt: f.addedAt, row: rowOf(f.key) ?? null })), combos: c.combos.map(comboDto) });
  const bump = (c: Case) => { c.updatedAt = now(); };
  const open = (c: Case) => { c.openedAt = Math.max(now(), ...[...cases.values()].map(x => x.openedAt + 1)); };
  const addFace = (c: Case, k: FaceKeyDto, origin: CaseOriginDto) => {
    const id = kOf(k);
    if (c.faces.has(id)) return false;
    if (c.faces.size >= MAX_FACES) throw invalid(`Case tối đa ${MAX_FACES} font`);
    c.faces.set(id, { key: { ...k }, origin, note: null, addedAt: now() });
    return true;
  };
  const renumber = (c: Case, order: Combo[]) => { c.combos = order; order.forEach((b, i) => { b.idx = i + 1; }); };
  const newCombo = (c: Case, name: string | null, from: Combo | null): Combo => {
    if (c.combos.length >= MAX_COMBOS) throw invalid(`Case tối đa ${MAX_COMBOS} Combo`);
    const idx = c.combos.length + 1;
    const b: Combo = { id: ++comboSeq, caseId: c.id, idx, name: name?.trim() || `Combo ${idx}`, isMain: c.combos.length === 0, note: null, slots: from ? from.slots.map(s => ({ ...s })) : [] };
    c.combos.push(b);
    return b;
  };
  const summary = (c: Case): CaseSummaryDto => ({ id: c.id, name: c.name, faceCount: c.faces.size, comboCount: c.combos.length, updatedAt: c.updatedAt, openedAt: c.openedAt, deletedAt: c.deletedAt });

  return {
    async createCase(name, brief, settings) {
      const n = name ?? `Case ${cases.size + 1}`;
      if (!n.trim()) throw invalid('tên Case rỗng');
      const t = now();
      const c: Case = { id: ++seq, name: n.trim(), brief: brief ?? null, settings: settings ?? { vi: true, mode: null }, viewState: {}, createdAt: t, updatedAt: t, openedAt: t, deletedAt: null, faces: new Map(), combos: [] };
      cases.set(c.id, c);
      open(c);
      return dto(c);
    },
    async listCases(includeDeleted = false) { return [...cases.values()].filter(c => includeDeleted || c.deletedAt == null).sort((a, b) => b.openedAt - a.openedAt).map(summary); },
    async openCase(id) { const c = live(id); open(c); return dto(c); },
    async getCase(id) { return dto(live(id)); },
    async updateCase(id, p) {
      const c = live(id);
      if (p.name != null) { if (!p.name.trim()) throw invalid('tên Case rỗng'); c.name = p.name.trim(); }
      if (p.clearBrief) c.brief = null; else if (p.brief) c.brief = p.brief;
      if (p.settings) c.settings = p.settings;
      if (p.viewState !== undefined && p.viewState !== null) c.viewState = p.viewState;
      bump(c);
      return dto(c);
    },
    async deleteCase(id) { const c = cases.get(id); if (!c) throw new CmdError('not_found', `Case #${id}`); c.deletedAt ??= now(); },
    async restoreCase(id) { const c = cases.get(id); if (!c) throw new CmdError('not_found', `Case #${id}`); c.deletedAt = null; open(c); return dto(c); },
    async addToCase(caseId, faces, origin = 'added'): Promise<AddToCaseDto> {
      let created = false;
      let c: Case;
      if (caseId != null) c = live(caseId);
      else {
        const latest = [...cases.values()].filter(x => x.deletedAt == null).sort((a, b) => b.openedAt - a.openedAt)[0];
        if (latest) c = latest; else { c = cases.get((await this.createCase()).id)!; created = true; }
      }
      const before = c.faces.size;
      const snapshot = new Map(c.faces);
      try { faces.forEach(f => addFace(c, f, origin)); } catch (e) { c.faces = snapshot; throw e; }
      const added = c.faces.size - before;
      bump(c); open(c);
      return { caseId: c.id, added, alreadyIn: faces.length - added, created };
    },
    async removeFromCase(caseId, faces) {
      const c = live(caseId);
      let n = 0;
      for (const f of faces) {
        c.combos.forEach(b => { b.slots = b.slots.filter(s => kOf(s.key) !== kOf(f)); });
        if (c.faces.delete(kOf(f))) n++;
      }
      bump(c);
      return n;
    },
    async createCombo(caseId, name, fromCombo) {
      const c = live(caseId);
      let from: Combo | null = null;
      if (fromCombo != null) { from = comboOf(fromCombo).b; if (from.caseId !== caseId) throw invalid('Combo nguồn thuộc Case khác'); }
      const b = newCombo(c, name ?? null, from);
      bump(c);
      return comboDto(b);
    },
    async updateCombo(id, p) {
      const { c, b } = comboOf(id);
      if (p.name != null) { if (!p.name.trim()) throw invalid('tên Combo rỗng'); b.name = p.name.trim(); }
      if (p.clearNote) b.note = null; else if (p.note != null) b.note = p.note;
      if (p.isMain === true) c.combos.forEach(x => { x.isMain = x === b; });
      else if (p.isMain === false) { b.isMain = false; if (!c.combos.some(x => x.isMain)) c.combos[0].isMain = true; }
      bump(c);
      return comboDto(b);
    },
    async deleteCombo(id) {
      const { c, b } = comboOf(id);
      renumber(c, c.combos.filter(x => x !== b));
      if (b.isMain && c.combos[0]) c.combos[0].isMain = true;
      bump(c);
    },
    async reorderCombos(caseId, ids) {
      const c = live(caseId);
      if (ids.length !== c.combos.length || new Set(ids).size !== ids.length || !ids.every(i => c.combos.some(b => b.id === i))) throw invalid('danh sách Combo không khớp Case');
      renumber(c, ids.map(i => c.combos.find(b => b.id === i)!));
      bump(c);
    },
    async setComboSlot(comboId, role, face, note, style) {
      if (!ROLES.includes(role)) throw invalid(`vai không hợp lệ: ${role}`);
      const { c, b } = comboOf(comboId);
      if (face) {
        const prev = b.slots.find(s => s.role === role);
        addFace(c, face, 'added');
        b.slots = [...b.slots.filter(s => s.role !== role), { role, key: { ...face }, note: note ?? prev?.note ?? null, style: style ?? prev?.style ?? {} }];
      } else b.slots = b.slots.filter(s => s.role !== role);
      bump(c);
      return comboDto(b);
    },
    async createCombos(caseId, combos) {
      const c = live(caseId);
      const snap = { combos: c.combos.map(b => ({ ...b, slots: [...b.slots] })), faces: new Map(c.faces) };
      try {
        return combos.map(nc => {
          const b = newCombo(c, nc.name, null);
          nc.slots.forEach(s => { if (!ROLES.includes(s.role)) throw invalid(`vai không hợp lệ: ${s.role}`); addFace(c, s.face, 'added'); b.slots.push({ role: s.role, key: { ...s.face }, note: null, style: {} }); });
          return comboDto(b);
        });
      } catch (e) { c.combos = snap.combos; c.faces = snap.faces; throw e; }
    },
  };
}
