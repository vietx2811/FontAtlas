import { describe, expect, it } from 'vitest';
import type { PlanItemDto } from '../../bindings';
import { createMockApi } from '../mock';
import { resolveHooks } from '../mockResolve';
import { CmdError } from '../types';

const req = (it: PlanItemDto, over: Partial<{ keeperPath: string; loserPaths: string[]; memberPaths: string[] }> = {}) => ({
  items: [{ kind: it.kind, key: it.key, keeperPath: it.keeper.path, loserPaths: it.losers.map(l => l.path), memberPaths: it.memberPaths, ...over }],
  action: 'hide' as const, explicitTrashPaths: [] as string[], mode: 'chosen' as const, joinBatch: null as string | null,
});

describe('mock resolve mirrors the backend contract', () => {
  it('plans: safe groups carry no review, review groups say why, every group is in memberPaths', async () => {
    const api = createMockApi();
    const all = await api.resolvePreview({ type: 'all' });
    const safe = await api.resolvePreview({ type: 'all_safe' });
    expect(all.items).toHaveLength(6);
    expect(safe.items.every(i => i.autoSafe && i.review === null)).toBe(true);
    expect(safe.needsReview).toBe(2);
    expect(all.items.filter(i => !i.autoSafe).every(i => i.review !== null)).toBe(true);
    for (const i of all.items) expect(new Set(i.memberPaths)).toEqual(new Set([i.keeper.path, ...i.losers.map(l => l.path)]));
    expect(all.hideCount).toBe(all.items.reduce((n, i) => n + i.losers.length, 0));
    await expect(api.resolvePreview({ type: 'group', kind: 'versions', key: 'nope' })).rejects.toBeInstanceOf(CmdError);
  });

  it('apply hides by file location: the group disappears from duplicateGroups and the summary', async () => {
    const api = createMockApi();
    const before = (await api.duplicateSummary()).total;
    const it = (await api.resolvePreview({ type: 'all_safe' })).items[0];
    const r = await api.resolveApply(req(it));
    expect(r.done).toHaveLength(1); expect(r.failed).toEqual([]);
    expect((await api.duplicateSummary()).total).toBe(before - 1);
    expect(await api.resolveList(false)).toHaveLength(1);
  });

  it('refuses a stale plan, a keeper that is also a loser, and untrashable or un-ticked files', async () => {
    const api = createMockApi();
    const items = (await api.resolvePreview({ type: 'all' })).items;
    const it = items[0];
    expect((await api.resolveApply(req(it, { memberPaths: [it.keeper.path] }))).failed[0].code).toBe('plan_stale');
    expect((await api.resolveApply(req(it, { loserPaths: [it.keeper.path] }))).failed[0].code).toBe('keeper_conflict');
    const sys = items.find(i => i.losers.some(l => l.trash.blocked === 'system_path'))!;
    const t = await api.resolveApply({ ...req(sys), action: 'trash' });
    expect(t.failed[0]).toMatchObject({ code: 'not_allowed', detail: 'system_path' });
    const exp = items.find(i => i.losers.some(l => l.trash.needsExplicit))!;
    expect((await api.resolveApply({ ...req(exp), action: 'trash' })).failed[0]).toMatchObject({ code: 'not_allowed', detail: 'needs_explicit' });
    const ok = await api.resolveApply({ ...req(exp), action: 'trash', explicitTrashPaths: exp.losers.map(l => l.path) });
    expect(ok.done).toHaveLength(1); expect(ok.done[0].trashPath).toBeTruthy();
    expect(await api.resolveList(false)).toHaveLength(1);
  });

  it('all_safe mode recomputes on the "backend" and ignores what the UI sent', async () => {
    const api = createMockApi();
    const items = (await api.resolvePreview({ type: 'all' })).items;
    const review = items.find(i => !i.autoSafe)!;
    const r = await api.resolveApply({ ...req(review), mode: 'all_safe' });
    expect(r.done).toEqual([]); expect(r.failed[0].code).toBe('not_auto_safe');
  });

  it('undo restores the group; undone decisions only show when asked; trash undo asks for a rescan; double undo is refused', async () => {
    const api = createMockApi();
    const it = (await api.resolvePreview({ type: 'all_safe' })).items[0];
    const before = (await api.duplicateSummary()).total;
    const { done } = await api.resolveApply({ ...req(it), action: 'hide' });
    const u = await api.resolveUndo(done.map(d => d.id));
    expect(u.done).toHaveLength(1); expect(u.needsRescan).toBe(false);
    expect((await api.duplicateSummary()).total).toBe(before);
    expect(await api.resolveList(false)).toEqual([]);
    expect(await api.resolveList(true)).toHaveLength(1);
    expect((await api.resolveUndo(done.map(d => d.id))).failed[0].code).toBe('already_undone');
    const exp = (await api.resolvePreview({ type: 'all' })).items.find(i => i.losers.some(l => l.trash.needsExplicit))!;
    const tr = await api.resolveApply({ ...req(exp), action: 'trash', explicitTrashPaths: exp.losers.map(l => l.path) });
    expect((await api.resolveUndo(tr.done.map(d => d.id))).needsRescan).toBe(true);
  });

  it('undoing an older decision that a newer one depends on is refused with newer_resolution_depends', async () => {
    const api = createMockApi();
    const it = (await api.resolvePreview({ type: 'group', kind: 'formats', key: 'Oswald|Regular|400|false' })).items[0];
    const [a, b, c] = [it.keeper, ...it.losers].map(m => m.path);
    const one = await api.resolveApply(req(it, { keeperPath: a, loserPaths: [b] }));
    await api.resolveApply(req(it, { keeperPath: c, loserPaths: [a], memberPaths: it.memberPaths.filter(p => p !== b) }));
    const r = await api.resolveUndo(one.done.map(d => d.id));
    expect(r.done).toEqual([]); expect(r.failed[0].code).toBe('newer_resolution_depends');
    expect(await api.resolveList(false)).toHaveLength(2);
  });

  it('ignore / unignore / ignored list: an ignored group leaves duplicateGroups and plans, and comes back', async () => {
    const api = createMockApi();
    const g = (await api.duplicateGroups())[0];
    await api.duplicateIgnore(g.kind, g.key); await api.duplicateIgnore(g.kind, g.key);
    expect((await api.duplicateGroups()).some(x => x.key === g.key)).toBe(false);
    expect((await api.resolvePreview({ type: 'all' })).items.some(i => i.key === g.key)).toBe(false);
    const ign = await api.duplicateIgnored();
    expect(ign).toHaveLength(1); expect(ign[0].group.key).toBe(g.key);
    await api.duplicateUnignore(g.kind, g.key);
    expect(await api.duplicateIgnored()).toEqual([]);
    expect((await api.duplicateGroups()).some(x => x.key === g.key)).toBe(true);
  });

  it('source priority: default is insertion order, the head you send wins, unknown or repeated ids are an error and change nothing', async () => {
    const api = createMockApi();
    expect(await api.sourcePriority()).toEqual([1, 2, 3]);
    expect(await api.setSourcePriority([3])).toEqual([3, 1, 2]);
    await expect(api.setSourcePriority([1, 1])).rejects.toBeInstanceOf(CmdError);
    await expect(api.setSourcePriority([9])).rejects.toBeInstanceOf(CmdError);
    expect(await api.sourcePriority()).toEqual([3, 1, 2]);
  });

  it('the source order decides the keeper, and the plan follows when it changes', async () => {
    const api = createMockApi();
    const find = async () => (await api.resolvePreview({ type: 'group', kind: 'versions', key: 'Roboto-Regular' })).items[0];
    expect((await find()).keeper.sourceId).toBe(1);
    await api.setSourcePriority([2]);
    const after = await find();
    expect(after.keeper.sourceId).toBe(2); expect(after.losers[0].sourceId).toBe(1);
    expect(resolveHooks(api)).toBeTruthy();
  });

  it('joinBatch puts a second request in the first request\'s batch; one undo reverts both; wrong batches are errors that change nothing', async () => {
    const api = createMockApi();
    const items = (await api.resolvePreview({ type: 'all_safe' })).items;
    const first = await api.resolveApply({ ...req(items[0]), action: 'hide' });
    const second = await api.resolveApply({ ...req(items[1]), joinBatch: first.batchId });
    expect(second.batchId).toBe(first.batchId);
    expect(new Set((await api.resolveList(false)).map(r => r.batchId)).size).toBe(1);
    await expect(api.resolveApply({ ...req(items[2]), joinBatch: 'nope' })).rejects.toMatchObject({ code: 'batch_not_found' });
    expect(await api.resolveList(false)).toHaveLength(2);
    const u = await api.resolveUndo([...first.done, ...second.done].map(d => d.id));
    expect(u.done).toHaveLength(2);
    await expect(api.resolveApply({ ...req(items[2]), joinBatch: first.batchId })).rejects.toMatchObject({ code: 'batch_undone' });
  });
});
