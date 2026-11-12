// Mock in-memory của Case + Combo (cùng hành vi với backend user.db v6): xoá mềm, Case mặc định = vừa mở,
// Combo đầu là chính, đánh số lại liên tục, đặt font vào vai tự đưa font vào Case.
import { CmdError } from './types';
import type { AddToCaseDto, Api, CaseDto, CaseOriginDto, CaseSummaryDto, ComboDto, ComboRoleDto } from './index';
import type { FaceKeyDto, FaceRowDto } from '../bindings';
import { comboAssessment } from './mockAnalysis';

const ROLES: ComboRoleDto[] = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote'];
const MAX_FACES = 500, MAX_COMBOS = 50, MAX_NAME = 120, MAX_ROLE = 40, MAX_ROLES = 30;
const kOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;

type Slot = { role: ComboRoleDto; key: FaceKeyDto; note: string | null; style: unknown };
// `roles` = mọi vai theo thứ tự (kể cả vai rỗng giữ chỗ); `slots` = vai CÓ font. Cùng ngữ nghĩa với user.db v11 (combo_slots).
type Combo = { id: number; caseId: number; idx: number; name: string; isMain: boolean; note: string | null; slots: Slot[]; roles: { name: string; ord: number }[] };
type Case = Omit<CaseDto, 'faces' | 'combos'> & { faces: Map<string, { key: FaceKeyDto; origin: CaseOriginDto; note: string | null; addedAt: number }>; combos: Combo[] };

type CaseApi = Pick<Api,
  'createCase' | 'listCases' | 'openCase' | 'getCase' | 'updateCase' | 'deleteCase' | 'restoreCase' | 'addToCase' | 'removeFromCase' |
  'caseCompare' | 'openCaseFromPeek' | 'createCombo' | 'updateCombo' | 'deleteCombo' | 'reorderCombos' | 'setComboSlot' | 'createCombos' | 'addComboRole' | 'removeComboRole' | 'renameComboRole' | 'reorderComboRoles'>;

export function createMockCases(rowOf: (k: FaceKeyDto) => FaceRowDto | undefined): CaseApi {
  const cases = new Map<number, Case>();
  let seq = 0, comboSeq = 0, tick = 0;
  const now = () => 1_700_000_000_000 + ++tick;
  const live = (id: number) => { const c = cases.get(id); if (!c || c.deletedAt != null) throw new CmdError('not_found', `Case #${id}`); return c; };
  const comboOf = (id: number) => { for (const c of cases.values()) { const b = c.combos.find(x => x.id === id); if (b) { if (c.deletedAt != null) throw new CmdError('not_found', `Case #${c.id} đã xoá`); return { c, b }; } } throw new CmdError('not_found', `Combo #${id}`); };
  const invalid = (m: string) => new CmdError('invalid_arg', m);
  const cleanName = (n: string, what: string) => { const t = n.trim(); if (!t || [...t].length > MAX_NAME) throw invalid(`tên ${what} rỗng hoặc quá dài (≤ ${MAX_NAME})`); return t; };
  const sameRole = (a: string, b: string) => a === b || a.toLowerCase() === b.toLowerCase();
  const roleErr = (code: string, m: string) => new CmdError(code, m);
  const cleanRole = (n: string) => { const t = n.trim(); if (!t || [...t].length > MAX_ROLE || /\p{Cc}/u.test(t)) throw roleErr('role_name_invalid', `tên vai rỗng, quá dài (≤ ${MAX_ROLE}) hoặc có ký tự điều khiển`); return t; };
  const orderedRoles = (b: Combo) => [...b.roles].sort((x, y) => x.ord - y.ord || (x.name < y.name ? -1 : x.name > y.name ? 1 : 0));
  const nextOrd = (b: Combo, name: string) => { const i = ROLES.findIndex(r => sameRole(r, name)); if (i >= 0) return i; return b.roles.length ? Math.max(ROLES.length - 1, ...b.roles.map(r => r.ord)) + 1 : ROLES.length; };
  /** Tên vai đã có (đúng chữ đã lưu) hoặc tạo mới: kiểm tên + giới hạn. */
  const ensureRole = (b: Combo, name: string): string => {
    const have = b.roles.find(r => sameRole(r.name, name.trim()));
    if (have) return have.name;
    const t = cleanRole(name);
    if (b.roles.length >= MAX_ROLES) throw roleErr('role_limit', `Combo tối đa ${MAX_ROLES} vai`);
    b.roles.push({ name: t, ord: nextOrd(b, t) });
    return t;
  };
  const isMissing = (k: FaceKeyDto) => { const r = rowOf(k); return !r || r.missing; };
  const slotDto = (s: Slot) => ({ role: s.role, key: { ...s.key }, row: rowOf(s.key) ?? null, missing: isMissing(s.key), note: s.note, style: s.style });
  const comboDto = (b: Combo): ComboDto => ({ id: b.id, caseId: b.caseId, idx: b.idx, name: b.name, isMain: b.isMain, note: b.note, slots: orderedRoles(b).flatMap(r => b.slots.filter(s => s.role === r.name)).map(slotDto), roles: orderedRoles(b).map(r => r.name) });
  const dto = (c: Case): CaseDto => ({ ...c, faces: [...c.faces.values()].map(f => ({ key: { ...f.key }, origin: f.origin, note: f.note, addedAt: f.addedAt, row: rowOf(f.key) ?? null, missing: isMissing(f.key) })), combos: c.combos.map(comboDto) });
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
    const b: Combo = { id: ++comboSeq, caseId: c.id, idx, name: name == null ? `Combo ${idx}` : cleanName(name, 'Combo'), isMain: c.combos.length === 0, note: null, slots: from ? from.slots.map(s => ({ ...s })) : [], roles: from ? from.roles.map(r => ({ ...r })) : [] };
    c.combos.push(b);
    return b;
  };
  const summary = (c: Case): CaseSummaryDto => ({ id: c.id, name: c.name, faceCount: c.faces.size, comboCount: c.combos.length, updatedAt: c.updatedAt, openedAt: c.openedAt, deletedAt: c.deletedAt });

  const api: CaseApi = {
    async createCase(name, brief, settings) {
      const n = cleanName(name ?? `Case ${cases.size + 1}`, 'Case');
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
      if (p.name != null) c.name = cleanName(p.name, 'Case');
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
      if (p.name != null) b.name = cleanName(p.name, 'Combo');
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
      const { c, b } = comboOf(comboId);
      const snapRoles = b.roles.map(r => ({ ...r }));
      try {
        role = ensureRole(b, role);
        if (face) {
          const prev = b.slots.find(s => s.role === role);
          addFace(c, face, 'added');
          b.slots = [...b.slots.filter(s => s.role !== role), { role, key: { ...face }, note: note ?? prev?.note ?? null, style: style ?? prev?.style ?? {} }];
        } else b.slots = b.slots.filter(s => s.role !== role); // chỉ gỡ font: vai ở lại (rỗng giữ chỗ)
      } catch (e) { b.roles = snapRoles; throw e; }
      bump(c);
      return comboDto(b);
    },
    async addComboRole(comboId, name) {
      const { c, b } = comboOf(comboId);
      const t = cleanRole(name);
      if (b.roles.some(r => sameRole(r.name, t))) throw roleErr('role_duplicate', `Combo đã có vai ${t}`);
      if (b.roles.length >= MAX_ROLES) throw roleErr('role_limit', `Combo tối đa ${MAX_ROLES} vai`);
      b.roles.push({ name: t, ord: b.roles.length ? Math.max(...b.roles.map(r => r.ord)) + 1 : 0 });
      bump(c);
      return comboDto(b);
    },
    async removeComboRole(comboId, role) {
      const { c, b } = comboOf(comboId);
      const have = b.roles.find(r => sameRole(r.name, role.trim()));
      if (!have) throw roleErr('role_not_found', `Combo không có vai ${role}`);
      b.roles = b.roles.filter(r => r !== have);
      b.slots = b.slots.filter(s => s.role !== have.name);
      bump(c);
      return comboDto(b);
    },
    async renameComboRole(comboId, role, newName) {
      const { c, b } = comboOf(comboId);
      const t = cleanRole(newName);
      const have = b.roles.find(r => sameRole(r.name, role.trim()));
      if (!have) throw roleErr('role_not_found', `Combo không có vai ${role}`);
      if (b.roles.some(r => r !== have && sameRole(r.name, t))) throw roleErr('role_duplicate', `Combo đã có vai ${t}`);
      b.slots.forEach(s => { if (s.role === have.name) s.role = t; });
      have.name = t;
      bump(c);
      return comboDto(b);
    },
    async reorderComboRoles(comboId, roles) {
      const { c, b } = comboOf(comboId);
      const order: { name: string; ord: number }[] = [];
      for (const r of roles) {
        const have = b.roles.find(x => sameRole(x.name, r.trim()));
        if (!have) throw roleErr('role_set_mismatch', `vai ${r} không có trong Combo`);
        if (order.includes(have)) throw roleErr('role_set_mismatch', `vai ${r} xuất hiện hai lần`);
        order.push(have);
      }
      if (order.length !== b.roles.length) throw roleErr('role_set_mismatch', 'danh sách vai phải gồm đủ mọi vai hiện có');
      order.forEach((r, i) => { r.ord = i; });
      bump(c);
      return comboDto(b);
    },
    async createCombos(caseId, combos) {
      const c = live(caseId);
      const snap = { combos: c.combos.map(b => ({ ...b, slots: [...b.slots], roles: b.roles.map(r => ({ ...r })) })), faces: new Map(c.faces) };
      try {
        return combos.map(nc => {
          const b = newCombo(c, nc.name, null);
          nc.slots.forEach(s => {
            if (b.roles.some(r => sameRole(r.name, s.role.trim()))) throw roleErr('role_duplicate', `vai trùng trong một Combo: ${s.role}`);
            const role = ensureRole(b, s.role);
            addFace(c, s.face, 'added'); b.slots.push({ role, key: { ...s.face }, note: null, style: {} });
          });
          return comboDto(b);
        });
      } catch (e) { c.combos = snap.combos; c.faces = snap.faces; throw e; }
    },
    async caseCompare(caseId, comboIds) {
      const c = live(caseId);
      const picked = comboIds.length ? comboIds.map(id => { const b = c.combos.find(x => x.id === id); if (!b) throw new CmdError('not_found', `Combo #${id} không thuộc Case`); return b; }) : c.combos;
      if (picked.length > 8) throw invalid('so sánh tối đa 8 Combo');
      return {
        caseId,
        columns: picked.map(b => {
          const dtoC = comboDto(b);
          const present = dtoC.slots.filter(s => !s.missing && s.row);
          // neo = vai đầu còn font; vai khác dùng font khác neo được so với neo (thiếu font ⇒ missing_font)
          const anchor = present[0];
          const relations = anchor ? dtoC.slots.filter(s => s.role !== anchor.role && kOf(s.key) !== kOf(anchor.key)).map(s => (s.missing || !s.row
            ? { from: anchor.role, to: s.role, assessment: null, reason: 'missing_font', reasonDetail: null }
            : { from: anchor.role, to: s.role, assessment: comboAssessment(anchor.row!, s.row, anchor.row!.faceId), reason: null, reasonDetail: null })) : [];
          const rows = present.map(s => s.row!);
          return {
            combo: dtoC, relations, fontCount: new Set(b.slots.map(s => kOf(s.key))).size, missingCount: dtoC.slots.filter(s => s.missing).length,
            worstViMissing: rows.length ? Math.max(...rows.map(r => r.viMissing)) : null, licenses: [...new Set(rows.map(r => r.licenseClass))].sort(),
          };
        }),
      };
    },
    async openCaseFromPeek(p) {
      const live0 = [...cases.values()].filter(x => x.deletedAt == null).sort((a, b) => b.openedAt - a.openedAt)[0];
      let created = false;
      let c: Case;
      if (p.caseId != null) c = live(p.caseId);
      else if (live0) c = live0;
      else { c = cases.get((await api.createCase('Case 1', undefined, p.settings)).id)!; created = true; }
      const before = { faces: new Map(c.faces), combos: c.combos.map(b => ({ ...b, slots: [...b.slots], roles: b.roles.map(r => ({ ...r })) })) };
      try {
        addFace(c, p.face, 'peek');
        if (p.candidate) addFace(c, p.candidate, 'peek');
      } catch (e) {
        c.faces = before.faces; c.combos = before.combos;
        if (created) cases.delete(c.id);
        throw e;
      }
      let main = c.combos.find(b => b.isMain);
      if (!main) main = newCombo(c, null, null);
      const role: ComboRoleDto = p.as === 'heading' ? 'display' : 'body';
      const held = main.slots.find(s => s.role === role);
      let occupied = null;
      if (held && kOf(held.key) !== kOf(p.face) && !p.replace) occupied = { role, key: { ...held.key }, row: rowOf(held.key) ?? null };
      else { ensureRole(main, role); main.slots = [...main.slots.filter(s => s.role !== role), { role, key: { ...p.face }, note: null, style: {} }]; }
      bump(c); open(c);
      return { case: dto(c), comboId: main.id, anchorRole: role, otherRole: role === 'display' ? 'body' : 'display', trial: p.candidate ?? null, created, occupied };
    },
  };
  return api;
}
