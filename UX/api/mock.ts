// Backend giả cho dev (trình duyệt/`cargo tauri dev` không có 6a) và Vitest. Hành vi bám bảng lệnh 6a.
import { cssSize, type FaceRow, type PxBucket } from '../features/library-grid/types';
import { createMockLibraryApi } from '../features/library-grid/api';
import type { Api } from './index';
import { createActivationSim } from './mockActivation';
import { createModelSim } from './mockModel';
import { VI_TOTAL, axesFor, pairFor, similarFor, viMissingFor } from './mockAnalysis';
import {
  CmdError, DEFAULT_UI_STATE, type ActivationStateDto, type CollectionDto, type FaceDetailDto, type FaceKeyDto,
  type FamilyStyleDto, type SettingsDto, type SourceDto, type TagDto, type UiStateDto,
} from './types';

const WEIGHTS: Array<[string, number]> = [['Thin', 100], ['ExtraLight', 200], ['Light', 300], ['Regular', 400], ['Medium', 500], ['SemiBold', 600], ['Bold', 700], ['ExtraBold', 800], ['Black', 900]];
// [family, nguồn, kiểu: 'ri' = đủ đứng+nghiêng 9 weight, 'r4' = Regular/Bold + nghiêng, '1' = một kiểu, viMissing]
const FAMILIES: Array<[string, number, 'ri' | 'r4' | '1', number]> = [
  ['Public Sans', 1, 'ri', 0], ['Roboto', 1, 'r4', 0], ['Inter', 1, 'ri', 0], ['Be Vietnam Pro', 1, 'ri', 0],
  ['Playfair Display', 1, 'r4', 0], ['Source Serif 4', 1, 'r4', 0], ['Fraunces', 2, 'r4', 3], ['DM Serif Display', 2, '1', 0],
  ['Bebas Neue', 2, '1', 12], ['Pacifico', 2, '1', 21], ['Lobster', 2, '1', 18], ['Oswald', 2, 'r4', 5],
  ['IBM Plex Mono', 2, 'r4', 0], ['JetBrains Mono', 1, 'r4', 0], ['Cormorant', 2, 'r4', 2], ['Abril Fatface', 3, '1', 9],
];

export function mockRows(): FaceRow[] {
  const rows: FaceRow[] = [];
  let id = 1;
  const push = (family: string, src: number, sub: string, w: number, italic: boolean, viMissing: number) => {
    const n = id++;
    rows.push({
      faceId: n, hashHex: n.toString(16).padStart(64, '0'), faceIndex: 0, family, subfamily: sub, fullName: `${family} ${sub}`,
      weightClass: w, widthClass: 5, isItalic: italic, isVariable: false, viStatus: viMissing ? 'partial' : 'full',
      supportsVi: viMissing === 0, viMissing, licenseClass: 'ofl',
      path: `/Users/me/Fonts/src${src}/${family.replace(/ /g, '')}-${sub.replace(/ /g, '')}.ttf`,
      fileSize: 180_000 + n * 1000, missing: false, isFavorite: false, addedAt: n,
    });
  };
  for (const [family, src, kind, miss] of FAMILIES) {
    if (kind === '1') push(family, src, 'Regular', 400, false, miss);
    else if (kind === 'r4') for (const [s, w] of [['Regular', 400], ['Bold', 700]] as const) { push(family, src, s, w, false, miss); push(family, src, `${s === 'Regular' ? '' : s + ' '}Italic`.trim(), w, true, miss); }
    else for (const [s, w] of WEIGHTS) { push(family, src, s, w, false, miss); push(family, src, `${s} Italic`, w, true, miss); }
  }
  return rows;
}

// Họ font hệ thống macOS cho mock: để lưới dev thấy từng họ khác nhau thay vì một font dự phòng.
const SYSTEM_FONTS: Record<string, string> = {
  'Public Sans': 'Helvetica Neue', 'Roboto': 'Arial', 'Inter': 'Avenir Next', 'Be Vietnam Pro': 'Gill Sans',
  'Playfair Display': 'Didot', 'Source Serif 4': 'Georgia', 'Fraunces': 'Baskerville', 'DM Serif Display': 'Bodoni 72',
  'Bebas Neue': 'Impact', 'Pacifico': 'Snell Roundhand', 'Lobster': 'Chalkduster', 'Oswald': 'Futura',
  'IBM Plex Mono': 'Courier New', 'JetBrains Mono': 'Menlo', 'Cormorant': 'Palatino', 'Abril Fatface': 'Copperplate',
};
const fold = (s: string) => s.normalize('NFD').replace(/\p{Mn}/gu, '').replace(/đ/gi, 'd').toLowerCase();
const keyOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;
const ok = <T,>(v: T): Promise<T> => Promise.resolve(structuredClone(v));

/** Ảnh preview giả: SVG chữ đen trên nền trong suốt (dùng làm mask như PNG alpha thật). */
function svgPreview(text: string, px: PxBucket, family: string, weight: number, italic: boolean): string {
  const { width, height } = cssSize(px);
  const esc = (s: string) => s.replace(/[<>&"']/g, c => `&#${c.charCodeAt(0)};`);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><text x="4" y="${height * 0.68}" font-size="${px / 2}" font-family="${esc(SYSTEM_FONTS[family] ?? family)}, sans-serif" font-weight="${weight}" font-style="${italic ? 'italic' : 'normal'}" fill="#000">${esc(text)}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

import type { BootStatusDto, EmbedStatusDto, LegacyStateDto, ModelStatusDto, OnboardingDto, ScanEvent } from './types';
export type MockOptions = { latencyMs?: number; tickMs?: number; /** Lần chạy đầu: chưa nguồn, chưa xác nhận giấy phép, chưa onboarding. */ firstRun?: boolean; legacy?: boolean; initial?: { model?: Partial<ModelStatusDto>; embed?: Partial<EmbedStatusDto> } };

export function createMockApi(o: MockOptions = {}): Api {
  const rows = mockRows();
  const byKey = new Map(rows.map(r => [keyOf(r), r]));
  const familyOf = new Map(rows.map(r => [r.hashHex, r.family]));
  const sourceOf = (r: FaceRow) => Number(/src(\d)/.exec(r.path ?? '')?.[1] ?? 1);

  let sources: SourceDto[] = o.firstRun ? [] : [
    { id: 1, kind: 'folder', name: 'Fonts', path: '/Users/me/Fonts/src1', enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false },
    { id: 2, kind: 'folder', name: 'Dự án khách', path: '/Users/me/Fonts/src2', enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false },
    { id: 3, kind: 'folder', name: 'Ổ ngoài', path: '/Volumes/Backup/src3', enabled: true, available: false, faceCount: 0, lastScan: null, scanning: false },
  ];
  const countOf = (id: number) => rows.filter(r => sourceOf(r) === id).length;
  const collections = new Map<number, CollectionDto & { faces: Set<string> }>();
  const tags = new Map<number, TagDto & { faces: Set<string> }>();
  let nextId = 10;
  let ui: UiStateDto = { ...DEFAULT_UI_STATE };
  const uiSettings = new Map<string, string>();
  const scanListeners = new Set<(e: ScanEvent) => void>();
  let scanning = false;
  let cacheBytes = 12 * 1024 * 1024;
  let onboarded = !o.firstRun;
  let legacy: LegacyStateDto = { found: !!o.legacy, preview: o.legacy ? { fonts: 1240, sources: 2, collections: 3, favorites: 17, tags: 9 } : null, imported: null, resolved: null, error: null };
  const legacyState = () => legacy;
  const boot = (): BootStatusDto => ({ phase: 'ready', storeError: null, libraryRecreated: false, userBackup: null, legacy });
  const settings: SettingsDto = { embedEnabled: false, licenseNoticeAck: !o.firstRun, uiLang: 'vi', sampleText: 'Phở bò Hà Nội', projectViDefault: false, keepActiveOnExit: false, rescanOnLaunch: true };

  const simulateScan = (id: number) => {
    const t = o.tickMs ?? 400; const emit = (e: ScanEvent) => scanListeners.forEach(l => l(e));
    scanning = true;
    emit({ type: 'queued', sourceId: id });
    [0.25, 0.5, 0.75, 1].forEach((f, i) => setTimeout(() => {
      emit({ type: 'progress', sourceId: id, progress: { phase: 'analyzing', done: Math.round(f * 200), total: 200, facesWritten: Math.round(f * 180), notDownloaded: 0 } });
      emit({ type: 'facesAdded', sourceId: id, count: 45 });
      if (f === 1) {
        emit({ type: 'finished', sourceId: id, stats: { status: 'ok', seen: 200, unchanged: 0, relinked: 0, analyzed: 195, failed: 3, missing: 0, unreadable: [], notDownloaded: 0 } });
        emit({ type: 'idle' }); scanning = false;
      }
    }, (i + 1) * t));
  };
  const act = createActivationSim({
    rowOf: k => byKey.get(keyOf(k)),
    collectionFaces: id => [...(collections.get(id)?.faces ?? [])].map(k => { const [h, i] = k.split(':'); return { hashHex: h, faceIndex: Number(i) }; }),
  });
  const sim = createModelSim({ tickMs: o.tickMs, initial: o.initial, embedEnabled: () => settings.embedEnabled });
  const lib = createMockLibraryApi(rows, { latencyMs: o.latencyMs });
  const delay = async () => { if (o.latencyMs) await new Promise(r => setTimeout(r, o.latencyMs)); };
  const findRow = (k: FaceKeyDto) => { const r = byKey.get(keyOf(k)); if (!r) throw new CmdError('not_found', 'face not found'); return r; };

  return {
    ...lib,
    async queryFaces(q) {
      await delay();
      const t = q.text ? fold(q.text) : '';
      if (q.faceKeys && q.faceKeys.length > 2000) throw new CmdError('invalid_arg', 'faceKeys: at most 2000');
      const wanted = q.faceKeys ? new Map(q.faceKeys.map((k, i) => [keyOf(k), i])) : null;
      const col = q.collectionId != null ? collections.get(q.collectionId) : null;
      let hits = rows.filter(r =>
        (!wanted || wanted.has(keyOf(r))) &&
        (!t || fold(r.family).includes(t) || fold(r.fullName ?? '').includes(t)) &&
        (!q.sourceIds?.length || q.sourceIds.includes(sourceOf(r))) &&
        (!q.favoritesOnly || r.isFavorite) &&
        (q.collectionId == null || !!col?.faces.has(keyOf(r))) &&
        (!q.tagIds?.length || q.tagIds.every(id => tags.get(id)?.faces.has(keyOf(r)))) &&
        (q.vi !== 'full' || r.viMissing === 0) && (q.vi !== 'supported' || r.supportsVi));
      if (q.sort === 'match' && wanted) hits = hits.sort((a, b) => wanted.get(keyOf(a))! - wanted.get(keyOf(b))!);
      else hits = hits.sort((a, b) => fold(a.family).localeCompare(fold(b.family)) || (a.weightClass ?? 0) - (b.weightClass ?? 0) || +a.isItalic - +b.isItalic);
      const offset = q.offset ?? 0;
      const limit = Math.min(q.limit || 100, 1000);
      return { total: hits.length, offset, items: hits.slice(offset, offset + limit).map(r => ({ ...r })) };
    },
    previewUrl: (row, { text, px }) => {
      const r = byKey.get(keyOf(row));
      return svgPreview(text, px, familyOf.get(row.hashHex) ?? '', r?.weightClass ?? 400, !!r?.isItalic);
    },
    fontUrl: () => null,

    listSources: () => ok(sources.map(s => ({ ...s, faceCount: countOf(s.id) }))),
    async addSource(path) {
      const hit = sources.find(s => s.path && (path === s.path || path.startsWith(`${s.path}/`) || s.path.startsWith(`${path}/`)));
      if (hit) throw new CmdError('nested_source', 'nested source', { existingId: hit.id, existingPath: hit.path });
      const s: SourceDto = { id: nextId++, kind: 'folder', name: path.split('/').pop() || path, path, enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false };
      sources = [...sources, s];
      simulateScan(s.id);
      return ok(s);
    },
    async removeSource(id) { sources = sources.filter(s => s.id !== id); },
    async rescanSources() {},

    async getFamilyStyles(family) {
      const out: FamilyStyleDto[] = rows.filter(r => r.family === family).map(r => ({ face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, subfamily: r.subfamily, weight: r.weightClass, italic: r.isItalic, instance: null }));
      return ok(out);
    },
    async setFavorite(k, on) { findRow(k).isFavorite = on; },
    listCollections: () => ok([...collections.values()].map(({ faces, ...c }) => ({ ...c, itemCount: faces.size }))),
    async createCollection(name, color) { const id = nextId++; const c = { id, name, color, itemCount: 0, faces: new Set<string>() }; collections.set(id, c); return ok({ id, name, color, itemCount: 0 }); },
    async renameCollection(id, name) { const c = collections.get(id); if (!c) throw new CmdError('not_found', 'collection'); c.name = name; },
    async deleteCollection(id) {
      const faces = [...(collections.get(id)?.faces ?? [])].map(k => { const [h, i] = k.split(':'); return { hashHex: h, faceIndex: Number(i) }; });
      collections.delete(id);
      act.collectionChanged(faces.map(f => [f, id] as [typeof f, number]), 'collection_deleted');
    },
    async addToCollection(id, faces) { const c = collections.get(id); if (!c) throw new CmdError('not_found', 'collection'); faces.forEach(f => c.faces.add(keyOf(f))); },
    async removeFromCollection(id, faces) {
      const c = collections.get(id); if (!c) return;
      faces.forEach(f => c.faces.delete(keyOf(f)));
      act.collectionChanged(faces.map(f => [f, id] as [typeof f, number]), 'left_collection');
    },
    listTags: () => ok([...tags.values()].map(({ faces, ...t }) => ({ ...t, faceCount: faces.size }))),
    async tagsOf(k) { return ok([...tags.values()].filter(t => t.faces.has(keyOf(k))).map(({ faces, ...t }) => ({ ...t, faceCount: faces.size }))); },
    async createTag(name) { const id = nextId++; tags.set(id, { id, name, faceCount: 0, faces: new Set() }); return ok({ id, name, faceCount: 0 }); },
    async tagFaces(id, faces) { const t = tags.get(id); if (!t) throw new CmdError('not_found', 'tag'); faces.forEach(f => t.faces.add(keyOf(f))); },
    async untagFaces(id, faces) { const t = tags.get(id); if (t) faces.forEach(f => t.faces.delete(keyOf(f))); },
    async revealInFinder(k) { findRow(k); },
    getUiState: () => ok(ui),
    async setUiState(s) { ui = { ...s }; },

    async getFace(k, _lang): Promise<FaceDetailDto> {
      const row = findRow(k);
      return ok({
        row, summary: `${row.family} ${row.subfamily ?? ''}`.trim(), classification: /Serif|Didone|Fraunces|Cormorant|Abril|Playfair/.test(row.family) ? 'Serif' : /Mono/.test(row.family) ? 'Mono' : 'Sans', source: 'axes', hasImage: false,
        file: { format: 'ttf', container: null, outline: 'glyf', numGlyphs: 1200 + row.faceId * 7, sizeBytes: row.fileSize, path: row.path },
        metadata: {
          license: 'SIL Open Font License 1.1', licenseUrl: 'https://openfontlicense.org', designer: 'Studio Mock', manufacturer: 'Mock Foundry',
          version: 'Version 2.001', copyright: `© 2024 The ${row.family} Project Authors`, vendorUrl: null,
          variableAxes: [], namedInstances: [], scripts: ['latn', 'vietnamese'], opszRange: null, features: { gsub: ['ccmp', 'liga'], gpos: ['kern', 'mark'] },
        },
        vietnamese: {
          status: row.viStatus, supports: row.supportsVi, missing: viMissingFor(row.viMissing), total: VI_TOTAL,
          precomposedComposite: row.viMissing === 0, gposMark: true, gposMkmk: row.faceId % 2 === 0, gsubCcmp: true, dongSign: true,
          stackedMarksTopEm: 1.08, stackedMarksOverflow: row.faceId % 4 === 1, stackStyle: row.faceId % 4 === 1 ? 'tight' : null,
          markWeightRatio: { key: 'mark_weight_ratio', value: 0.82, quality: 'measured', reason: null, method: 'outline', percentile: 55 },
        },
        axes: axesFor(row),
        similarTop: similarFor(row, rows, 3),
      });
    },
    async similar(k, o) { const r = findRow(k); return ok(similarFor(r, rows, o.k)); },
    async pairCandidates(k, o) { const r = findRow(k); return ok(pairFor(r, rows, o)); },
    getSettings: () => ok(settings),
    async setSettings(patch) {
      const was = settings.embedEnabled;
      Object.assign(settings, patch);
      if (patch.embedEnabled !== undefined && patch.embedEnabled !== was) sim.embedEnabledChanged(settings.embedEnabled);
      return ok(settings);
    },
    ...sim,
    async getSetting(key) { return uiSettings.get(key) ?? null; },
    async setSetting(key, value) {
      if (!/^ui\.[a-z0-9_.-]{1,60}$/.test(key)) throw new CmdError('invalid_arg', 'only ui.* keys');
      if (new TextEncoder().encode(value).length > 65_536) throw new CmdError('invalid_arg', 'value over 64 KB');
      uiSettings.set(key, value);
    },
    // Dữ liệu và hệ thống
    async backupUserDb() { return '/Users/me/Library/Application Support/com.vietx.fontatlas/backups/user-20261106.db'; },
    async previewCacheInfo() { return ok({ bytes: cacheBytes, entries: cacheBytes ? 340 : 0 }); },
    async clearPreviewCache() { cacheBytes = 0; },
    async listProblems() { return []; },
    async retryProblems() { return 0; },
    // Khởi động, onboarding, quét
    async startupStatus() { return ok(boot()); },
    async bootSubscribe(l) {
      const phases = ['scanning', 'indexing', 'activation', 'ready'] as const;
      const timers = phases.map((phase, i) => setTimeout(() => l({ type: 'phase', phase }), (i + 1) * (o.tickMs ?? 400) / 4));
      return () => timers.forEach(clearTimeout);
    },
    async onboardingState() {
      return ok<OnboardingDto>({
        step: uiSettings.get('onboarding_step') || null, hasSources: sources.length > 0, legacy: legacyState(), licenseNoticeAck: settings.licenseNoticeAck,
        embedEnabled: settings.embedEnabled, modelState: (await sim.modelStatus()).state, activationSupported: true, activationOnboarded: onboarded, keepActiveOnExit: settings.keepActiveOnExit,
        fontManagers: [{ name: 'RightFont', installed: false, activeCount: null }, { name: 'FontBase', installed: true, activeCount: null }],
      });
    },
    async setOnboardingStep(step) { uiSettings.set('onboarding_step', step ?? ''); },
    async legacyImport() { legacy = { ...legacyState(), imported: { fonts: 1240, sources: 2, collections: 3, recoveredCollections: 0, badTags: 0, skippedSources: 0 } }; return ok(legacy); },
    async legacyRetry() { return ok(legacyState()); },
    async scanStatus() { return ok({ running: scanning, paused: false, currentSourceId: null, queued: [] }); },
    async scanPause() {}, async scanResume() {}, async scanCancel() {},
    async scanSubscribe(l) { scanListeners.add(l); return () => { scanListeners.delete(l); }; },
    ...act,
  };
}
