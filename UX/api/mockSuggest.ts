// Mock của các lệnh gợi ý Desk (G5/G6) cho dev không có Tauri và Vitest. Hành vi bám hợp đồng của 08
// (docs/team/plans/2026-11-12-desk-suggest-api-contract.md): không có điểm số, mã lý do + tham số, mã lỗi, cùng seed cùng kết quả.
import { CmdError } from './types';
import type { Api } from './index';
import type { ComboSuggestionDto, FaceKeyDto, FaceRowDto, FitDto, RecentViewDto, WhyDto } from '../bindings';

type SuggestApi = Pick<Api, 'deskSuggest' | 'suggestPartners' | 'suggestCombos' | 'roleFit' | 'recentViews' | 'recordView' | 'clearRecentViews'>;
const PAIR_STYLES = ['contrast', 'superfamily', 'same_structure', 'same_era', 'display_vs_neutral', 'harmony'];
const PROFILES = ['body', 'caption', 'heading', 'display', 'quote', 'label', 'numerals'];
const PRESETS = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote', 'label', 'numerals'];
const kOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;
const key = (r: FaceRowDto): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
const why = (code: string, params: Array<[string, number]> = []): WhyDto => ({ code, params: params.map(([k, value]) => ({ key: k, value })) });
const bad = (code: string) => new CmdError(code, code);

export function createMockSuggest(deps: { rows: () => FaceRowDto[]; caseFaces: (caseId: number) => Promise<FaceKeyDto[]> }): SuggestApi {
  const recent: Array<{ face: FaceKeyDto; viewedAt: number }> = [];
  let tick = 0;
  const profileOf = (name: string, profile: string | null): string | null => {
    const k = name.trim().toLowerCase();
    return PRESETS.includes(k) ? (k === 'subheading' || k === 'accent' ? 'heading' : k) : profile && PROFILES.includes(profile) ? profile : null;
  };
  const fitOf = (r: FaceRowDto, profile: string | null): FitDto => (profile === null ? 'unknown' : (r.weightClass ?? 400) % 300 === 0 ? 'good' : 'ok');
  const mocksort = (rows: FaceRowDto[], seed = 0) => [...rows].sort((a, b) => (a.family + seed).localeCompare(b.family + seed) || a.faceId - b.faceId);
  const usable = () => deps.rows().filter(r => !r.missing);
  const passFilter = (r: FaceRowDto, f: { classes?: string[] | null; text?: string | null } | null, projectVi: boolean) =>
    (!projectVi || r.supportsVi) && (!f?.text || `${r.family} ${r.subfamily ?? ''}`.toLowerCase().includes(f.text.toLowerCase()));
  return {
    async deskSuggest(req) {
      if (!['fits_role', 'similar', 'pinned'].includes(req.source)) throw bad('source_unknown');
      if (req.source === 'similar' && !req.anchor) throw bad('anchor_required');
      if (req.source === 'pinned' && req.caseId == null) throw bad('case_required');
      const profile = profileOf(req.targetRole.name, req.targetRole.profile);
      const inCase = req.source === 'pinned' ? new Set((await deps.caseFaces(req.caseId!)).map(kOf)) : null;
      let pool = inCase ? deps.rows().filter(r => inCase.has(kOf(key(r)))) : usable();
      if (req.anchor) pool = pool.filter(r => kOf(key(r)) !== kOf(req.anchor!));
      pool = pool.filter(r => passFilter(r, req.filter as never, req.projectVi));
      pool = mocksort(pool);
      const items = pool.slice(req.offset, req.offset + req.limit).map(r => ({
        face: key(r), row: r, pinned: req.source === 'pinned',
        fit: req.source === 'fits_role' ? fitOf(r, profile) : null,
        reasons: req.source === 'fits_role' ? (profile === null ? [why('role_unknown')] : [why('x_height_ok')]) : req.source === 'similar' ? [why('similar_x_height_em')] : [],
      }));
      return { source: req.source, items, total: pool.length };
    },
    async suggestPartners(anchor, req) {
      for (const s of req.styles) if (!PAIR_STYLES.includes(s)) throw bad('style_unknown');
      const target = profileOf(req.targetRole.name, req.targetRole.profile);
      if (target === null) return { items: [], total: 0 };
      const pool = mocksort(usable().filter(r => r.family !== deps.rows().find(x => kOf(key(x)) === kOf(anchor))?.family && passFilter(r, req.filter as never, req.projectVi)));
      const items = pool.slice(req.offset, req.offset + req.limit).map((r, i) => ({
        face: key(r), row: r, gate: i % 5 === 4 ? 'controlled' : 'safe', styles: [req.styles[0] ?? PAIR_STYLES[i % 3]],
        roleFit: fitOf(r, target), styleFit: 'ok' as FitDto, reasons: [why('class_not_contrasting'), why('x_height_ok')], warnings: i % 7 === 6 ? [why('xheight_mismatch', [['delta', 0.1]])] : [],
      }));
      return { items, total: pool.length };
    },
    async suggestCombos(req) {
      for (const s of req.styles) if (!PAIR_STYLES.includes(s)) throw bad('style_unknown');
      if (!req.roles.length || req.roles.some(r => profileOf(r.name, r.profile) === null)) throw bad('roles_invalid');
      if (req.scope === 'case' && req.caseId == null) throw bad('case_required');
      if (!['library', 'case'].includes(req.scope)) throw bad('scope_unknown');
      const inCase = req.scope === 'case' ? new Set((await deps.caseFaces(req.caseId!)).map(kOf)) : null;
      const pool = mocksort(inCase ? deps.rows().filter(r => inCase.has(kOf(key(r)))) : usable(), req.seed);
      const fams = [...new Set(pool.map(r => r.family))];
      const out: ComboSuggestionDto[] = [];
      for (let i = 0; i < fams.length && out.length < Math.max(3, Math.min(6, req.k)); i++) {
        const used = new Set<string>();
        const slots = req.roles.map((role, ri) => {
          const lock = req.locked.find(l => l.role.name.toLowerCase() === role.name.toLowerCase());
          const row = lock ? deps.rows().find(r => kOf(key(r)) === kOf(lock.face))! : pool.find(r => r.family === fams[(i + ri) % fams.length] && !used.has(r.family)) ?? pool[(i + ri) % pool.length];
          used.add(row.family);
          return { role, face: key(row), row, fit: fitOf(row, profileOf(role.name, role.profile)), locked: !!lock };
        });
        if (new Set(slots.map(s => s.row.family)).size < Math.min(2, slots.length)) continue;
        out.push({ slots, style: req.styles[0] ?? PAIR_STYLES[out.length % 3], reasons: [why('class_not_contrasting')], warnings: [], viMissing: slots.reduce((n, s) => n + (s.row.supportsVi ? 0 : s.row.viMissing), 0), seed: req.seed });
      }
      return out;
    },
    async roleFit(face, role) {
      const r = deps.rows().find(x => kOf(key(x)) === kOf(face));
      const p = profileOf(role.name, role.profile);
      return { fit: r ? fitOf(r, p) : 'unknown', profile: p, reasons: p === null ? [why('role_unknown')] : [why('x_height_ok')] };
    },
    async recentViews(limit): Promise<RecentViewDto[]> {
      return recent.slice(0, limit).map(v => ({ face: v.face, row: deps.rows().find(r => kOf(key(r)) === kOf(v.face)) ?? null, viewedAt: v.viewedAt }));
    },
    async recordView(face) { const i = recent.findIndex(v => kOf(v.face) === kOf(face)); if (i >= 0) recent.splice(i, 1); recent.unshift({ face, viewedAt: 1_700_000_000_000 + ++tick }); recent.length = Math.min(recent.length, 200); },
    async clearRecentViews() { const n = recent.length; recent.length = 0; return n; },
  };
}
