import { describe, expect, it } from 'vitest';
import { createMockApi } from '../mock';
import { CmdError } from '../types';

const role = (name: string, profile: string | null = null) => ({ name, profile, size: null });
const req = (over: object = {}) => ({ source: 'fits_role', anchor: null, caseId: null, targetRole: role('Body'), filter: null, projectVi: false, limit: 20, offset: 0, ...over }) as never;
const code = async (p: Promise<unknown>) => { try { await p; return null; } catch (e) { return e instanceof CmdError ? e.code : 'other'; } };

describe('mock suggest (bám hợp đồng 08)', () => {
  it('deskSuggest: nguồn lạ, thiếu anchor/case → mã lỗi; fits_role có fit, vai lạ không hồ sơ → unknown + role_unknown', async () => {
    const api = createMockApi();
    expect(await code(api.deskSuggest(req({ source: 'x' })))).toBe('source_unknown');
    expect(await code(api.deskSuggest(req({ source: 'similar' })))).toBe('anchor_required');
    expect(await code(api.deskSuggest(req({ source: 'pinned' })))).toBe('case_required');
    const ok = await api.deskSuggest(req());
    expect(ok.items.length).toBeGreaterThan(0); expect(ok.items.every(i => i.fit !== null)).toBe(true);
    const odd = await api.deskSuggest(req({ targetRole: role('Tagline') }));
    expect(odd.items[0].fit).toBe('unknown'); expect(odd.items[0].reasons[0].code).toBe('role_unknown');
    const withProfile = await api.deskSuggest(req({ targetRole: role('Tagline', 'heading') }));
    expect(withProfile.items[0].fit).not.toBe('unknown');
  });
  it('pinned trả đúng font của Case; không có trường điểm số nào', async () => {
    const api = createMockApi(); const [a] = (await api.queryFaces({ limit: 1 })).items;
    const c = await api.createCase('X'); await api.addToCase(c.id, [{ hashHex: a.hashHex, faceIndex: a.faceIndex }]);
    const p = await api.deskSuggest(req({ source: 'pinned', caseId: c.id }));
    expect(p.total).toBe(1); expect(p.items[0].pinned).toBe(true);
    expect(JSON.stringify(p)).not.toMatch(/score/i);
  });
  it('suggestCombos: cùng seed cùng kết quả, khoá giữ nguyên, lỗi mã đúng', async () => {
    const api = createMockApi(); const rows = (await api.queryFaces({ limit: 50 })).items; const lock = rows[0];
    const r = { styles: [], roles: [role('Heading'), role('Body')], locked: [{ role: role('Heading'), face: { hashHex: lock.hashHex, faceIndex: lock.faceIndex } }], scope: 'library', caseId: null, projectVi: false, k: 4, seed: 7 } as never;
    const a = await api.suggestCombos(r), b = await api.suggestCombos(r);
    expect(JSON.stringify(a)).toBe(JSON.stringify(b)); expect(a.length).toBeGreaterThanOrEqual(3);
    expect(a.every(c => c.slots.find(s => s.role.name === 'Heading')!.locked)).toBe(true);
    expect(await code(api.suggestCombos({ ...(r as object), roles: [role('Tagline')] } as never))).toBe('roles_invalid');
    expect(await code(api.suggestCombos({ ...(r as object), scope: 'case' } as never))).toBe('case_required');
    expect(await code(api.suggestCombos({ ...(r as object), styles: ['nope'] } as never))).toBe('style_unknown');
  });
  it('recent: ghi, mới nhất đứng đầu, không trùng, xoá trả số dòng', async () => {
    const api = createMockApi(); const [a, b] = (await api.queryFaces({ limit: 2 })).items;
    const ka = { hashHex: a.hashHex, faceIndex: a.faceIndex }, kb = { hashHex: b.hashHex, faceIndex: b.faceIndex };
    await api.recordView(ka); await api.recordView(kb); await api.recordView(ka);
    const r = await api.recentViews(10); expect(r.map(x => x.face.hashHex)).toEqual([ka.hashHex, kb.hashHex]);
    expect(await api.clearRecentViews()).toBe(2); expect(await api.recentViews(10)).toEqual([]);
  });
});
