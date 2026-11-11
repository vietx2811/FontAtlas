/** Mock "giải quyết trùng lặp": mô phỏng backend (kế hoạch, ẩn theo vị trí tệp, hoàn tác, bỏ qua nhóm, thứ tự nguồn). */
import type {
  DuplicateGroupDto, DuplicateKindDto, IgnoredGroupDto, PlanItemDto, PlanMemberDto, ResolutionDto, ResolveFailureDto, ResolvePlanDto,
  ResolveReasonDto, ResolveRequestDto, ResolveResultDto, ResolveScopeDto,
} from '../bindings';
import { CmdError } from './types';

export type MockResolveDeps = { baseGroups: () => Promise<DuplicateGroupDto[]>; sourceIds: () => number[]; };

const FORMAT_RANK = ['otf', 'ttf', 'woff', 'woff2', 'sfnt'];
const sourceOfPath = (p: string): number => Number(/src(\d)/.exec(p)?.[1] ?? 1);
const gkey = (kind: string, key: string) => `${kind}\u0000${key}`;
const reason = (code: string, args: Record<string, string> = {}): ResolveReasonDto => ({ code, args });
const base = (p: string) => p.slice(p.lastIndexOf('/') + 1);

export function createMockResolve(deps: MockResolveDeps) {
  let priority: number[] = [];
  const hidden = new Map<string, number>(); // path → id quyết định
  const ignored = new Map<string, number>();
  const resolutions: ResolutionDto[] = [];
  let nextId = 1, nextBatch = 1, clock = 1_700_000_000;
  /** Test: ép một đường dẫn bản thua thất bại với `code`. */
  const failPaths = new Map<string, string>();
  /** Test: số lần gọi và yêu cầu cuối. */
  const log = { applied: [] as ResolveRequestDto[] };

  const prio = (): number[] => { const all = deps.sourceIds(); return [...priority.filter(i => all.includes(i)), ...all.filter(i => !priority.includes(i))]; };
  // Font hệ thống luôn là bản kém ưu tiên nhất (không thể dọn khỏi máy), nên thành bản thua.
  const rank = (p: string) => { if (p.startsWith('/Library/') || p.startsWith('/System/')) return 50; const i = prio().indexOf(sourceOfPath(p)); return i < 0 ? 99 : i; };

  const member = (g: DuplicateGroupDto, i: number): PlanMemberDto => {
    const f = g.faces[i];
    const path = f.path ?? f.row.path ?? `/Users/me/Fonts/src1/${f.row.family}-${i}.ttf`;
    const sys = path.startsWith('/Library/') || path.startsWith('/System/');
    const explicit = path.includes('/Library/Fonts/') && !sys;
    return {
      face: { hashHex: f.row.hashHex, faceIndex: f.row.faceIndex }, family: f.row.family, style: f.row.subfamily, path, format: f.format,
      version: f.version, revision: f.revision, modified: f.modified, numGlyphs: 1200 + i * 40, fileSize: f.row.fileSize, viMissing: f.row.viMissing,
      isVariable: f.row.isVariable, sourceId: sourceOfPath(path), sourceName: `Nguồn ${sourceOfPath(path)}`, sourceKind: sys ? 'system' : 'directory',
      active: false,
      trash: sys ? { allowed: false, needsExplicit: false, blocked: 'system_path' } : { allowed: true, needsExplicit: explicit, blocked: null },
    };
  };

  const visible = (g: DuplicateGroupDto) => g.faces.filter((f, i) => !hidden.has(member(g, i).path));
  async function liveGroups(): Promise<DuplicateGroupDto[]> {
    return (await deps.baseGroups()).map(g => ({ ...g, faces: g.faces.filter((_, i) => !hidden.has(member(g, i).path)) }))
      .filter(g => g.faces.length >= 2 && !ignored.has(gkey(g.kind, g.key)));
  }

  function planFor(g: DuplicateGroupDto): PlanItemDto {
    const ms = g.faces.map((_, i) => member(g, i));
    const order = ms.map((m, i) => ({ m, i }));
    let keepIdx: number, why: ResolveReasonDto, review: ResolveReasonDto | null = null;
    const warnings: ResolveReasonDto[] = [];
    if (g.kind === 'same_file') {
      order.sort((a, b) => rank(a.m.path) - rank(b.m.path) || a.m.path.length - b.m.path.length);
      keepIdx = order[0].i;
      why = rank(order[0].m.path) < rank(order[1].m.path) ? reason('preferred_source') : reason('same_content_tidiest_path');
    } else if (g.kind === 'versions') {
      const nw = g.faces.findIndex(f => f.isNewest);
      if (g.newestKnown && nw >= 0) {
        keepIdx = nw;
        const other = ms.find((_, i) => i !== nw)!;
        why = reason('newer_version', { keep: String(ms[nw].revision ?? '?'), other: String(other.revision ?? '?') });
        warnings.push(reason('may_reflow'));
      } else { keepIdx = order.sort((a, b) => rank(a.m.path) - rank(b.m.path))[0].i; why = reason('tie'); review = reason('newest_unknown'); }
    } else {
      const r = (f: string) => { const i = FORMAT_RANK.indexOf(f); return i < 0 ? 99 : i; };
      keepIdx = order.sort((a, b) => r(a.m.format) - r(b.m.format))[0].i;
      why = reason('preferred_format', { format: ms[keepIdx].format.toUpperCase() });
    }
    const keeper = ms[keepIdx];
    const losers = ms.filter((_, i) => i !== keepIdx);
    if (losers.some(l => l.format === 'sfnt') && !review) review = reason('collection_file');
    return { kind: g.kind, key: g.key, keeper, losers, reason: why, autoSafe: review === null, review, warnings, memberPaths: ms.map(m => m.path) };
  }

  async function preview(scope: ResolveScopeDto): Promise<ResolvePlanDto> {
    const groups = await liveGroups();
    const all = groups.map(planFor);
    let items: PlanItemDto[];
    if (scope.type === 'group') {
      const g = groups.find(x => x.kind === scope.kind && x.key === scope.key);
      if (!g) throw new CmdError('not_found', 'group not found');
      items = [planFor(g)];
    } else items = scope.type === 'all' ? all : all.filter(p => p.autoSafe);
    return {
      items, needsReview: scope.type === 'all_safe' ? all.filter(p => !p.autoSafe).length : items.filter(p => !p.autoSafe).length,
      keepCount: items.length, hideCount: items.reduce((n, p) => n + p.losers.length, 0),
      reclaimableBytes: items.reduce((n, p) => n + p.losers.reduce((s, l) => s + l.fileSize, 0), 0),
    };
  }

  async function apply(req: ResolveRequestDto): Promise<ResolveResultDto> {
    log.applied.push(structuredClone(req));
    const batchId = `b${nextBatch++}`;
    const done: ResolutionDto[] = [], failed: ResolveFailureDto[] = [];
    const groups = await liveGroups();
    for (const it of req.items) {
      const g = groups.find(x => x.kind === it.kind && x.key === it.key);
      if (!g) { failed.push({ path: it.keeperPath ?? '', code: 'not_found', detail: null }); continue; }
      const plan = planFor(g);
      let keeperPath = it.keeperPath, losers = it.loserPaths;
      if (req.mode === 'all_safe') {
        if (!plan.autoSafe) { failed.push({ path: plan.keeper.path, code: 'not_auto_safe', detail: null }); continue; }
        keeperPath = plan.keeper.path; losers = plan.losers.map(l => l.path);
      } else if (it.memberPaths.slice().sort().join('\n') !== plan.memberPaths.slice().sort().join('\n')) {
        failed.push({ path: keeperPath, code: 'plan_stale', detail: null }); continue;
      } else if (losers.includes(keeperPath)) { failed.push({ path: keeperPath, code: 'keeper_conflict', detail: null }); continue; }
      const k = [plan.keeper, ...plan.losers].find(m => m.path === keeperPath);
      if (!k) { failed.push({ path: keeperPath, code: 'keeper_not_in_group', detail: null }); continue; }
      for (const lp of losers) {
        const l = [plan.keeper, ...plan.losers].find(m => m.path === lp);
        if (!l) { failed.push({ path: lp, code: 'loser_not_in_group', detail: null }); continue; }
        const forced = failPaths.get(lp);
        if (forced) { failed.push({ path: lp, code: forced, detail: null }); continue; }
        if (req.action === 'trash') {
          if (!l.trash.allowed) { failed.push({ path: lp, code: 'not_allowed', detail: l.trash.blocked }); continue; }
          if (l.trash.needsExplicit && !req.explicitTrashPaths.includes(lp)) { failed.push({ path: lp, code: 'not_allowed', detail: 'needs_explicit' }); continue; }
        }
        const r: ResolutionDto = {
          id: nextId++, batchId, kind: it.kind, groupKey: it.key, keeper: k.face, keeperPath, loser: l.face, loserPath: lp, action: req.action,
          trashPath: req.action === 'trash' ? `/Users/me/.Trash/${base(lp)}` : null, reason: req.mode === 'all_safe' ? plan.reason : keeperPath === plan.keeper.path ? plan.reason : reason('chosen_by_user'),
          wasActive: false, createdAt: clock++, undoneAt: null, linksMoved: 0, family: l.family, style: l.style, status: 'done', keeperActivated: false, canUndo: true,
        };
        resolutions.push(r); hidden.set(lp, r.id); done.push(r);
      }
    }
    return { batchId, done: structuredClone(done), failed, needsRescan: false };
  }

  async function undo(ids: number[]): Promise<ResolveResultDto> {
    const done: ResolutionDto[] = [], failed: ResolveFailureDto[] = [];
    const todo = ids.map(id => resolutions.find(r => r.id === id)).filter((r): r is ResolutionDto => !!r).sort((a, b) => b.id - a.id);
    for (const id of ids) if (!resolutions.some(r => r.id === id)) failed.push({ path: '', code: 'not_found', detail: null });
    let rescan = false;
    for (const r of todo) {
      if (r.undoneAt !== null) { failed.push({ path: r.loserPath, code: 'already_undone', detail: null }); continue; }
      // Chuỗi A→B→C: bản giữ của quyết định này đã bị quyết định mới hơn ẩn.
      const dep = resolutions.find(x => x.id > r.id && x.undoneAt === null && x.loserPath === r.keeperPath && !ids.includes(x.id));
      if (dep) { failed.push({ path: r.loserPath, code: 'newer_resolution_depends', detail: null }); continue; }
      if (!r.canUndo) { failed.push({ path: r.loserPath, code: 'trash_emptied', detail: null }); continue; }
      r.undoneAt = clock++; r.canUndo = false; hidden.delete(r.loserPath);
      if (r.action === 'trash') rescan = true;
      done.push(structuredClone(r));
    }
    return { batchId: todo[0]?.batchId ?? '', done, failed, needsRescan: rescan };
  }

  return {
    liveGroups,
    methods: {
      resolvePreview: (scope: ResolveScopeDto) => preview(scope),
      resolveApply: apply,
      resolveUndo: undo,
      async resolveList(includeUndone: boolean) { return structuredClone(resolutions.filter(r => includeUndone || r.undoneAt === null).sort((a, b) => b.id - a.id)); },
      async duplicateIgnore(kind: DuplicateKindDto, key: string) { if (!ignored.has(gkey(kind, key))) ignored.set(gkey(kind, key), clock++); },
      async duplicateUnignore(kind: DuplicateKindDto, key: string) { ignored.delete(gkey(kind, key)); },
      async duplicateIgnored(): Promise<IgnoredGroupDto[]> {
        const all = await deps.baseGroups();
        return all.filter(g => ignored.has(gkey(g.kind, g.key))).map(g => ({ group: g, ignoredAt: ignored.get(gkey(g.kind, g.key))! })).sort((a, b) => b.ignoredAt - a.ignoredAt);
      },
      async sourcePriority() { return prio(); },
      async setSourcePriority(ids: number[]) {
        const all = deps.sourceIds();
        if (new Set(ids).size !== ids.length || ids.some(i => !all.includes(i))) throw new CmdError('invalid_arg', 'bad source priority');
        priority = ids; return prio();
      },
    },
    __failPaths: failPaths, __log: log, __visibleCount: (g: DuplicateGroupDto) => visible(g).length,
  };
}

/** Móc kiểm thử của mock: ép lỗi cho một đường dẫn, đọc lại yêu cầu đã gửi. */
export type MockResolveHooks = ReturnType<typeof createMockResolve>;
export const resolveHooks = (api: unknown): MockResolveHooks => (api as { __resolve: MockResolveHooks }).__resolve;
