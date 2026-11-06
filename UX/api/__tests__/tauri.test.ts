import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
let reply: (cmd: string) => unknown = () => null;
vi.mock('@tauri-apps/api/core', () => {
  class Channel<T> { onmessage: (m: T) => void = () => {}; }
  return {
    Channel,
    convertFileSrc: (name: string, scheme: string) => `${scheme}://localhost/${name}`,
    invoke: async (cmd: string, args: Record<string, unknown>) => {
      calls.push({ cmd, args });
      const r = reply(cmd);
      if (r instanceof Error) throw r;
      if (r && typeof r === 'object' && '__reject' in r) throw (r as { __reject: unknown }).__reject;
      return r;
    },
  };
});

import { makeRow } from '../../features/library-grid/api';
import { ContractError } from '../../features/library-grid/types';
import { createTauriApi, toCmdError, toFaceQueryDto } from '../tauri';
import { CmdError } from '../types';

const last = () => calls.at(-1)!;
beforeEach(() => { calls.length = 0; reply = () => null; });

describe('query_faces', () => {
  it('fills every FaceQueryDto field with the 6a defaults (camelCase, no missing keys)', () => {
    expect(toFaceQueryDto({})).toEqual({
      text: null, vi: null, license: [], variable: null, italic: null, sourceIds: [], favoritesOnly: false, collectionId: null, tagIds: [],
      includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 100, axisFilters: [], hard: [], rawAxisRanges: [], faceKeys: null,
    });
  });

  it('passes the UI filters through and treats blank text as no text', () => {
    const d = toFaceQueryDto({ text: '  ', vi: 'full', favoritesOnly: true, tagIds: [3], faceKeys: [{ hashHex: 'a', faceIndex: 1 }], sort: 'match', offset: 200, limit: 100 });
    expect(d).toMatchObject({ text: null, vi: 'full', favoritesOnly: true, tagIds: [3], faceKeys: [{ hashHex: 'a', faceIndex: 1 }], sort: 'match', offset: 200 });
    expect(toFaceQueryDto({ text: 'phở' }).text).toBe('phở');
  });

  it('sends {q} and validates the page against the Plan 5 contract', async () => {
    const row = makeRow(0);
    reply = () => ({ total: 1, offset: 0, items: [row] });
    const page = await createTauriApi().queryFaces({ text: 'a' });
    expect(last().cmd).toBe('query_faces');
    expect(Object.keys(last().args)).toEqual(['q']);
    expect(page.items[0].faceId).toBe(row.faceId);
    reply = () => ({ total: 1, offset: 0, items: [{ ...row, surprise: 1 }] });
    await expect(createTauriApi().queryFaces({})).rejects.toBeInstanceOf(ContractError);
  });
});

describe('round2 contracts', () => {
  it('sends physical raw filters to query_faces, axis_stats and family pagination to similar_families', async () => {
    const api = createTauriApi();
    const q = { italic: true, sourceIds: [2], rawAxisRanges: [{ axis: 'slant_deg', range: [-25.5, 15] as [number, number] }], offset: 6, limit: 6 };
    expect(toFaceQueryDto(q).rawAxisRanges).toEqual(q.rawAxisRanges);
    await api.axisStats(q);
    expect(last()).toEqual({ cmd: 'axis_stats', args: { q: toFaceQueryDto(q) } });
    const face = { hashHex: 'ab', faceIndex: 2 };
    await api.similarFamilies(face, { useImage: false }, q);
    expect(last()).toEqual({ cmd: 'similar_families', args: { face, opts: { weights: { proportion: 1, weight: 1, structure: 1, detail: 1 }, useImage: false, k: 12, viOnly: false, sourceIds: [], weightRange: null }, q: toFaceQueryDto(q) } });
  });
});

describe('errors', () => {
  it('rejections become CmdError with code, message and detail', async () => {
    reply = () => ({ __reject: { code: 'nested_source', message: 'nested', detail: { existingId: 4, existingPath: '/F' } } });
    const e = await createTauriApi().addSource('/F/x').catch(x => x);
    expect(e).toBeInstanceOf(CmdError);
    expect([e.code, e.message, e.detail]).toEqual(['nested_source', 'nested', { existingId: 4, existingPath: '/F' }]);
    expect(last()).toEqual({ cmd: 'add_source', args: { path: '/F/x' } });
  });

  it('a bare string or an Error becomes code internal', () => {
    expect(toCmdError('boom')).toMatchObject({ code: 'internal', message: 'boom' });
    expect(toCmdError(new Error('x'))).toMatchObject({ code: 'internal', message: 'x' });
  });
});

describe('command arguments (camelCase keys of the 6a table)', () => {
  it('simple commands', async () => {
    const api = createTauriApi();
    await api.removeSource(3); expect(last()).toEqual({ cmd: 'remove_source', args: { sourceId: 3 } });
    await api.rescanSources(); expect(last().args).toEqual({ sourceIds: null });
    await api.rescanSources([1, 2]); expect(last().args).toEqual({ sourceIds: [1, 2] });
    await api.setFavorite({ hashHex: 'h', faceIndex: 0 }, true); expect(last()).toEqual({ cmd: 'set_favorite', args: { face: { hashHex: 'h', faceIndex: 0 }, on: true } });
    await api.tagFaces(5, [{ hashHex: 'h', faceIndex: 0 }]); expect(Object.keys(last().args)).toEqual(['tagId', 'faces']);
    await api.createCollection('Menu', null); expect(last().args).toEqual({ name: 'Menu', color: null });
    await api.getFace({ hashHex: 'h', faceIndex: 0 }, 'vi'); expect(last()).toMatchObject({ cmd: 'get_face', args: { lang: 'vi' } });
    await api.setSetting('ui.recent_views', '[]'); expect(last()).toEqual({ cmd: 'set_setting', args: { key: 'ui.recent_views', value: '[]' } });
    await api.setOnboardingStep(null); expect(last()).toEqual({ cmd: 'set_onboarding_step', args: { step: null } });
  });

  it('set_settings sends a full patch with null for what did not change', async () => {
    reply = () => ({});
    await createTauriApi().setSettings({ embedEnabled: true });
    expect(last().args).toEqual({ patch: { embedEnabled: true, licenseNoticeAck: null, uiLang: null, sampleText: null, projectViDefault: null, keepActiveOnExit: null, rescanOnLaunch: null } });
  });

  it('similar fills the 6a defaults', async () => {
    await createTauriApi().similar({ hashHex: 'h', faceIndex: 0 });
    expect(last().args.opts).toEqual({ weights: { proportion: 1, weight: 1, structure: 1, detail: 1 }, useImage: true, k: 12, viOnly: false, sourceIds: [], weightRange: null });
    await createTauriApi().similar({ hashHex: 'h', faceIndex: 0 }, { k: 3, useImage: false });
    expect(last().args.opts).toMatchObject({ k: 3, useImage: false });
  });

  it('activation commands carry scope/proceed/force and a progress Channel', async () => {
    const api = createTauriApi();
    const faces = [{ hashHex: 'h', faceIndex: 0 }];
    const onProgress = vi.fn();
    await api.activationActivate(faces, { proceed: true, onProgress });
    expect(last().cmd).toBe('activation_activate');
    expect(last().args).toMatchObject({ faces, scope: null, proceed: true });
    const ch = last().args.onProgress as { onmessage(m: unknown): void };
    ch.onmessage({ done: 1, total: 2, hashHex: 'h' });
    expect(onProgress).toHaveBeenCalledWith({ done: 1, total: 2, hashHex: 'h' });
    await api.activationDeactivate(faces, { force: true });
    expect(last().args).toMatchObject({ faces, force: true });
    await api.activationActivateCollection(7, { scope: 'persistent' });
    expect(last().args).toMatchObject({ collectionId: 7, scope: 'persistent', proceed: false });
    await api.activationDeactivateCollection(7); expect(last().args).toMatchObject({ collectionId: 7 });
    await api.activationDeactivateAll(); expect(last().cmd).toBe('activation_deactivate_all');
    await api.activationSetKeep(true); expect(last()).toEqual({ cmd: 'activation_set_keep', args: { on: true } });
  });
});

describe('Channels', () => {
  it('subscribe registers first, delivers events, and stops after unsubscribe', async () => {
    const got: unknown[] = [];
    const off = await createTauriApi().embedSubscribe(e => got.push(e));
    expect(last().cmd).toBe('embed_subscribe');
    const ch = last().args.onEvent as { onmessage(m: unknown): void };
    ch.onmessage({ kind: 'finished', totalDone: 3 });
    off();
    ch.onmessage({ kind: 'finished', totalDone: 4 });
    expect(got).toEqual([{ kind: 'finished', totalDone: 3 }]);
  });

  it('model_download streams to the callback', async () => {
    const got: unknown[] = [];
    await createTauriApi().modelDownload(e => got.push(e));
    (last().args.onEvent as { onmessage(m: unknown): void }).onmessage({ type: 'ready' });
    expect(got).toEqual([{ type: 'ready' }]);
  });

  it('modelRedownload surfaces busy as a CmdError', async () => {
    reply = () => ({ __reject: { code: 'busy', message: 'download in progress' } });
    const e = await createTauriApi().modelRedownload(() => {}).catch(x => x);
    expect([e instanceof CmdError, e.code]).toEqual([true, 'busy']);
  });
});

describe('preview and font URLs', () => {
  it('go through the fontatlas-preview / fontatlas-font schemes; TTC faces have no font URL', () => {
    const api = createTauriApi();
    const u = api.previewUrl({ hashHex: 'ab', faceIndex: 0 }, { text: 'Phở', px: 48, generation: 2, rendererVersion: 'r1' });
    expect(u).toBe('fontatlas-preview://localhost/ab_0_48.png?t=Phở&g=2&r=r1'.replace('Phở', encodeURIComponent('Phở')));
    expect(api.fontUrl({ hashHex: 'ab', faceIndex: 0 })).toBe('fontatlas-font://localhost/ab_0.font');
    expect(api.fontUrl({ hashHex: 'ab', faceIndex: 1 })).toBeNull();
  });
});
