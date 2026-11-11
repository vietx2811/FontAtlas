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
      includeMissing: false, sort: 'auto', descending: false, offset: 0, limit: 100, axisFilters: [], hard: [], rawAxisRanges: [], faceKeys: null, classes: [], subclasses: [], traits: [], includeSecondary: false, confidentOnly: false, minConfidence: null,
      langs: [], langMode: 'all', badgeLang: null,
      weightClass: null, widthClass: null, varAxisMatch: { wght: true, wdth: true },
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

describe('case/combo commands', () => {
  it('maps every Api method to its command with camelCase args and explicit nulls', async () => {
    const api = createTauriApi();
    const f = { hashHex: 'ab', faceIndex: 0 };
    const cases: Array<[() => Promise<unknown>, string, unknown]> = [
      [() => api.createCase(), 'create_case', { name: null, brief: null, settings: null }],
      [() => api.listCases(), 'list_cases', { includeDeleted: false }],
      [() => api.addToCase(null, [f]), 'add_to_case', { caseId: null, faces: [f], origin: null }],
      [() => api.addToCase(3, [f], 'peek'), 'add_to_case', { caseId: 3, faces: [f], origin: 'peek' }],
      [() => api.createCombo(3), 'create_combo', { caseId: 3, name: null, fromCombo: null }],
      [() => api.setComboSlot(5, 'body', null), 'set_combo_slot', { comboId: 5, role: 'body', face: null, note: null, style: null }],
      [() => api.reorderCombos(3, [2, 1]), 'reorder_combos', { caseId: 3, ids: [2, 1] }],
      [() => api.briefStarter({ uses: ['poster'], tones: [], needs: { vi: true, installedOnly: false, italic: false, variable: false, openLicense: false }, text: null }), 'brief_starter', { brief: { uses: ['poster'], tones: [], needs: { vi: true, installedOnly: false, italic: false, variable: false, openLicense: false }, text: null }, caseId: null }],
      [() => api.caseCompare(3, [1, 2]), 'case_compare', { caseId: 3, comboIds: [1, 2], lang: 'vi' }],
      [() => api.updateCase(3, { clearBrief: true }), 'update_case', { id: 3, patch: { name: null, brief: null, clearBrief: true, settings: null, viewState: null } }],
    ];
    for (const [run, cmd, args] of cases) { await run(); expect(last()).toEqual({ cmd, args }); }
  });
});

describe('round2 contracts', () => {
  it('sends physical raw filters to query_faces, axis_stats and family pagination to similar_families', async () => {
    const api = createTauriApi();
    const q = { italic: true, sourceIds: [2], rawAxisRanges: [{ axis: 'slant_deg', range: [-25.5, 15] as [number, number] }], offset: 6, limit: 6 };
    expect(toFaceQueryDto(q).rawAxisRanges).toEqual(q.rawAxisRanges);
    await api.axisStats(q);
    await api.axisHistogram(q, 12);
    expect(last()).toEqual({ cmd: 'axis_histogram', args: { q: toFaceQueryDto(q), bins: 12 } });
    await api.axisStats(q);
    expect(last()).toEqual({ cmd: 'axis_stats', args: { q: toFaceQueryDto(q) } });
    const face = { hashHex: 'ab', faceIndex: 2 };
    await api.similarFamilies(face, { useImage: false }, q);
    expect(last()).toEqual({ cmd: 'similar_families', args: { face, opts: { weights: { proportion: 1, weight: 1, structure: 1, detail: 1 }, useImage: false, k: 12, viOnly: false, sourceIds: [], weightRange: null }, q: toFaceQueryDto(q) } });
  });
});

describe('classification commands', () => {
  it('maps to class_facets / set_classification_override / clear_classification_override', async () => {
    const api = createTauriApi();
    await api.classFacets({ classes: ['serif'], italic: true });
    expect(last()).toEqual({ cmd: 'class_facets', args: { q: toFaceQueryDto({ classes: ['serif'], italic: true }) } });
    const face = { hashHex: 'ab', faceIndex: 0 };
    await api.setClassificationOverride({ face, class: 'script', subclass: null });
    expect(last()).toEqual({ cmd: 'set_classification_override', args: { over: { face, class: 'script', subclass: null } } });
    await api.clearClassificationOverride(face);
    expect(last()).toEqual({ cmd: 'clear_classification_override', args: { face } });
  });
});

describe('compare_faces', () => {
  it('invokes the real command with lang and maps the fixture to the UI model', async () => {
    const wire = (await import('../../bindings/fixtures/compare_two.json')).default;
    reply = cmd => (cmd === 'compare_faces' ? wire : null);
    const faces = wire.faces.map(f => f.face);
    const c = await createTauriApi().compareFaces(faces);
    expect(last()).toEqual({ cmd: 'compare_faces', args: { faces, opts: { lang: 'vi' } } });
    expect(c.faces[0].face).toEqual(wire.baseline);
    expect(c.faces[0].extras?.vendorId).toBe(wire.faces[0].extras.vendorId);
  });
  it('rejects 1 or 9 faces without calling the backend and wraps backend errors', async () => {
    const api = createTauriApi();
    await expect(api.compareFaces([{ hashHex: 'a', faceIndex: 0 }])).rejects.toThrow();
    expect(calls).toHaveLength(0);
    reply = () => ({ __reject: { code: 'not_found', message: 'face' } });
    await expect(api.compareFaces([{ hashHex: 'a', faceIndex: 0 }, { hashHex: 'b', faceIndex: 0 }])).rejects.toMatchObject({ code: 'not_found' });
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
    expect(last().args).toEqual({ patch: { embedEnabled: true, licenseNoticeAck: null, uiLang: null, sampleText: null, projectViDefault: null, keepActiveOnExit: null, rescanOnLaunch: null, cloudPreviewDownload: null } });
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
    await api.activationSetScope([{ hashHex: 'aa', faceIndex: 0 }], 'persistent'); expect(last()).toEqual({ cmd: 'activation_set_scope', args: { faces: [{ hashHex: 'aa', faceIndex: 0 }], scope: 'persistent' } });
    await api.activationAvailability(); expect(last()).toEqual({ cmd: 'activation_availability', args: { faces: [] } });
    await api.activationAvailability([{ hashHex: 'aa', faceIndex: 1 }]); expect(last().args).toEqual({ faces: [{ hashHex: 'aa', faceIndex: 1 }] });
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

describe('pair_these', () => {
  it('invokes the real command with Vietnamese project flag, lang and maxCombos', async () => {
    const combos = (await import('../../bindings/fixtures/pair_these_combos.json')).default;
    reply = cmd => (cmd === 'pair_these' ? combos : null);
    const faces = [{ hashHex: 'a', faceIndex: 0 }, { hashHex: 'b', faceIndex: 0 }];
    const r = await createTauriApi().pairThese(faces, { projectVi: false, maxCombos: 2 });
    expect(last()).toEqual({ cmd: 'pair_these', args: { faces, opts: { projectVi: false, lang: 'vi', maxCombos: 2 } } });
    expect(r).toHaveLength(combos.length);
  });
  it('defaults to project Vietnamese on and 3 combos; validates 2–4 faces', async () => {
    reply = () => [];
    const api = createTauriApi();
    await api.pairThese([{ hashHex: 'a', faceIndex: 0 }, { hashHex: 'b', faceIndex: 0 }]);
    expect(last().args.opts).toEqual({ projectVi: true, lang: 'vi', maxCombos: 3 });
    await expect(api.pairThese([{ hashHex: 'a', faceIndex: 0 }])).rejects.toThrow();
  });
});

describe('save_png', () => {
  it('sends the suggested name and the PNG as base64 and returns the saved path (null when cancelled)', async () => {
    reply = cmd => (cmd === 'save_png' ? '/Users/me/Desktop/compare-a.png' : null);
    const api = createTauriApi();
    const png = new Blob([new Uint8Array([137, 80, 78, 71])], { type: 'image/png' });
    expect(await api.savePng(png, 'compare-a.png')).toBe('/Users/me/Desktop/compare-a.png');
    expect(last()).toEqual({ cmd: 'save_png', args: { suggestedName: 'compare-a.png', pngBase64: btoa(String.fromCharCode(137, 80, 78, 71)) } });
    reply = () => null;
    expect(await api.savePng(png, 'x.png')).toBeNull();
  });
});

describe('duplicates', () => {
  it('maps duplicate_summary and duplicate_groups with the full FaceQueryDto', async () => {
    const groups = (await import('../../bindings/fixtures/duplicate_groups.json')).default;
    const sum = (await import('../../bindings/fixtures/duplicate_summary.json')).default;
    reply = cmd => (cmd === 'duplicate_groups' ? groups : cmd === 'duplicate_summary' ? sum : null);
    const api = createTauriApi();
    const q = { text: 'play', italic: false };
    expect(await api.duplicateSummary(q)).toEqual(sum);
    expect(last()).toEqual({ cmd: 'duplicate_summary', args: { q: toFaceQueryDto(q) } });
    expect(await api.duplicateGroups(q)).toHaveLength(groups.length);
    expect(last()).toEqual({ cmd: 'duplicate_groups', args: { q: toFaceQueryDto(q) } });
  });
});

describe('missing_chars', () => {
  it('sends the face and the already-segmented graphemes', async () => {
    reply = () => ['龍'];
    await expect(createTauriApi().missingChars({ hashHex: 'ab', faceIndex: 0 }, ['a', '龍'])).resolves.toEqual(['龍']);
    expect(last()).toEqual({ cmd: 'missing_chars', args: { face: { hashHex: 'ab', faceIndex: 0 }, chars: ['a', '龍'] } });
  });
});

describe('reveal_location', () => {
  it('sends the exact path', async () => {
    reply = () => null;
    await createTauriApi().revealLocation('/Users/u/Downloads/A.ttf');
    expect(last()).toEqual({ cmd: 'reveal_location', args: { path: '/Users/u/Downloads/A.ttf' } });
  });
});

describe('taxonomy v2 query flags', () => {
  it('toFaceQueryDto sends includeSecondary / confidentOnly / minConfidence (explicit null when unset)', async () => {
    const { toFaceQueryDto } = await import('../tauri');
    const d = toFaceQueryDto({ classes: ['slab'], includeSecondary: true, confidentOnly: true, minConfidence: 0.9 });
    expect([d.includeSecondary, d.confidentOnly, d.minConfidence]).toEqual([true, true, 0.9]);
    const e = toFaceQueryDto({});
    expect([e.includeSecondary, e.confidentOnly, e.minConfidence]).toEqual([false, false, null]);
  });
});
