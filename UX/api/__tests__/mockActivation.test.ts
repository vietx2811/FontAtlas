import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../mock';
import { CmdError, type ActivationEvent, type FaceKeyDto } from '../types';

const key = (r: { hashHex: string; faceIndex: number }): FaceKeyDto => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });
async function rowOf(api: ReturnType<typeof createMockApi>, family: string) { return (await api.queryFaces({ text: family, limit: 50 })).items[0]; }

describe('mock activation (Plan 7 contract)', () => {
  it('state carries faces and reasons as {kind, collectionId}', async () => {
    const api = createMockApi();
    const r = await rowOf(api, 'Inter');
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(r)]);
    await api.activationActivate([key(r)]);
    await api.activationActivateCollection(col.id);
    const [s] = await api.activationState();
    expect(s).toMatchObject({ hashHex: r.hashHex, faces: [0], state: 'active', status: 'registered' });
    expect(s.reasons).toEqual([{ kind: 'manual' }, { kind: 'collection', collectionId: col.id }]);
  });

  it('activate then deactivate reports activated / deactivated, with progress per file', async () => {
    const api = createMockApi();
    const r = await rowOf(api, 'Inter');
    const onProgress = vi.fn();
    const a = await api.activationActivate([key(r)], { onProgress });
    expect(a.results[0].outcome).toEqual({ kind: 'activated' });
    expect(a.changed).toBe(1);
    expect(onProgress).toHaveBeenCalledWith({ done: 1, total: 1, hashHex: r.hashHex });
    expect((await api.activationActivate([key(r)])).results[0].outcome).toEqual({ kind: 'already_active' });
    expect((await api.activationDeactivate([key(r)])).results[0].outcome).toEqual({ kind: 'deactivated' });
    expect((await api.activationDeactivate([key(r)])).results[0].outcome).toEqual({ kind: 'already_inactive' });
  });

  it('a font held by a collection asks for force; force removes every reason', async () => {
    const api = createMockApi();
    const r = await rowOf(api, 'Inter');
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(r)]);
    await api.activationActivateCollection(col.id);
    const held = await api.activationDeactivate([key(r)]);
    expect(held.results[0].outcome).toEqual({ kind: 'held_by_collections', collectionIds: [col.id] });
    expect((await api.activationState())).toHaveLength(1);
    expect((await api.activationDeactivate([key(r)], { force: true })).results[0].outcome.kind).toBe('deactivated');
    expect(await api.activationState()).toHaveLength(0);
  });

  it('a name clash needs confirmation; proceed activates (preflight predicts it)', async () => {
    const api = createMockApi();
    const r = await rowOf(api, 'Roboto');
    const pre = await api.activationPreflight([key(r)]);
    expect(pre.needsConfirmation).toBe(1);
    const asked = await api.activationActivate([key(r)]);
    expect(asked.needsConfirmation).toBe(1);
    const o = asked.results[0].outcome;
    expect(o.kind === 'needs_confirmation' && o.conflicts[0]).toMatchObject({ owner: 'system', existingPath: expect.stringContaining('/Library/Fonts/') });
    expect(await api.activationState()).toHaveLength(0);
    expect((await api.activationActivate([key(r)], { proceed: true })).results[0].outcome).toEqual({ kind: 'activated' });
  });

  it('a file changed on disk fails with code file_changed', async () => {
    const api = createMockApi();
    const r = await rowOf(api, 'Lobster');
    const rep = await api.activationActivate([key(r)]);
    expect(rep.failed).toBe(1);
    expect(rep.results[0].outcome).toEqual({ kind: 'failed', code: 'file_changed', message: expect.any(String) });
  });

  it('leaving or deleting an activated collection auto-deactivates, and says why', async () => {
    const api = createMockApi();
    const [a, b] = (await api.queryFaces({ limit: 2 })).items;
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(a), key(b)]);
    await api.activationActivateCollection(col.id);
    const events: ActivationEvent[] = [];
    await api.activationSubscribe(e => events.push(e));
    await api.removeFromCollection(col.id, [key(a)]);
    const left = events.find(e => e.type === 'reconciled');
    expect(left && left.type === 'reconciled' && left.report.autoDeactivated).toEqual([{ hashHex: a.hashHex, faces: [0], removedReasons: [{ kind: 'collection', collectionId: col.id }], cause: 'left_collection' }]);
    events.length = 0;
    await api.deleteCollection(col.id);
    const del = events.find(e => e.type === 'reconciled');
    expect(del && del.type === 'reconciled' && del.report.autoDeactivated.map(x => x.cause)).toEqual(['collection_deleted']);
    expect(await api.activationState()).toHaveLength(0);
  });

  it('can simulate the OS refusing the auto-deactivation (autoDeactivateFailed, font stays registered)', async () => {
    const api = createMockApi();
    const [a] = (await api.queryFaces({ limit: 1 })).items;
    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(a)]);
    await api.activationActivateCollection(col.id);
    const events: ActivationEvent[] = [];
    await api.activationSubscribe(e => events.push(e));
    (api as unknown as { failNextAutoDeactivate: Set<string> }).failNextAutoDeactivate.add(a.hashHex);
    await api.deleteCollection(col.id);
    const rec = events.find(e => e.type === 'reconciled');
    expect(rec && rec.type === 'reconciled' && rec.report.autoDeactivateFailed).toHaveLength(1);
    expect((await api.activationState()).map(s => s.hashHex)).toEqual([a.hashHex]);
  });

  it('deactivate all clears everything', async () => {
    const api = createMockApi();
    const items = (await api.queryFaces({ limit: 3 })).items;
    await api.activationActivate(items.map(key));
    const rep = await api.activationDeactivateAll();
    expect(rep.changed).toBe(3);
    expect(await api.activationState()).toEqual([]);
  });
});

describe('mock model: redownload while downloading is busy', () => {
  it('rejects with CmdError busy and keeps downloading', async () => {
    const api = createMockApi();
    await api.modelDownload(() => {});
    const err = await api.modelRedownload(() => {}).catch(e => e);
    expect(err).toBeInstanceOf(CmdError);
    expect(err.code).toBe('busy');
    expect((await api.modelStatus()).state).toBe('downloading');
  });
});

describe('mock boot / onboarding / scan', () => {
  it('first run: no sources, no license ack, no step; later runs are settled', async () => {
    const first = await createMockApi({ firstRun: true }).onboardingState();
    expect([first.hasSources, first.licenseNoticeAck, first.step]).toEqual([false, false, null]);
    const later = await createMockApi().onboardingState();
    expect([later.hasSources, later.licenseNoticeAck]).toEqual([true, true]);
  });

  it('remembers the step and the legacy preview/import', async () => {
    const api = createMockApi({ firstRun: true, legacy: true });
    await api.setOnboardingStep('o3');
    expect((await api.onboardingState()).step).toBe('o3');
    expect((await api.onboardingState()).legacy.preview).toMatchObject({ sources: 2, favorites: 17 });
    expect((await api.legacyImport()).imported).toMatchObject({ fonts: 1240 });
  });

  it('adding a source streams scan events to idle', async () => {
    vi.useFakeTimers();
    const api = createMockApi({ firstRun: true, tickMs: 100 });
    const seen: string[] = [];
    await api.scanSubscribe(e => seen.push(e.type));
    await api.addSource('/Users/me/NewFonts');
    await vi.advanceTimersByTimeAsync(500);
    expect(seen[0]).toBe('queued');
    expect(seen).toContain('progress');
    expect(seen).toContain('facesAdded');
    expect(seen.slice(-2)).toEqual(['finished', 'idle']);
    vi.useRealTimers();
  });
});
