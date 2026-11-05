// Backend giả cho dev (trình duyệt/`cargo tauri dev` không có 6a) và Vitest. Hành vi bám bảng lệnh 6a.
import { cssSize, type FaceRow, type PxBucket } from '../features/library-grid/types';
import { createMockLibraryApi } from '../features/library-grid/api';
import type { Api } from './index';
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

const fold = (s: string) => s.normalize('NFD').replace(/\p{Mn}/gu, '').replace(/đ/gi, 'd').toLowerCase();
const keyOf = (k: FaceKeyDto) => `${k.hashHex}:${k.faceIndex}`;
const ok = <T,>(v: T): Promise<T> => Promise.resolve(structuredClone(v));

/** Ảnh preview giả: SVG chữ đen trên nền trong suốt (dùng làm mask như PNG alpha thật). */
function svgPreview(text: string, px: PxBucket, family: string): string {
  const { width, height } = cssSize(px);
  const esc = (s: string) => s.replace(/[<>&"']/g, c => `&#${c.charCodeAt(0)};`);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><text x="4" y="${height * 0.68}" font-size="${px / 2}" font-family="${esc(family)}, sans-serif" fill="#000">${esc(text)}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

export type MockOptions = { latencyMs?: number };

export function createMockApi(o: MockOptions = {}): Api {
  const rows = mockRows();
  const byKey = new Map(rows.map(r => [keyOf(r), r]));
  const familyOf = new Map(rows.map(r => [r.hashHex, r.family]));
  const sourceOf = (r: FaceRow) => Number(/src(\d)/.exec(r.path ?? '')?.[1] ?? 1);

  let sources: SourceDto[] = [
    { id: 1, kind: 'folder', name: 'Fonts', path: '/Users/me/Fonts/src1', enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false },
    { id: 2, kind: 'folder', name: 'Dự án khách', path: '/Users/me/Fonts/src2', enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false },
    { id: 3, kind: 'folder', name: 'Ổ ngoài', path: '/Volumes/Backup/src3', enabled: true, available: false, faceCount: 0, lastScan: null, scanning: false },
  ];
  const countOf = (id: number) => rows.filter(r => sourceOf(r) === id).length;
  const collections = new Map<number, CollectionDto & { faces: Set<string> }>();
  const tags = new Map<number, TagDto & { faces: Set<string> }>();
  const active = new Set<string>();
  let nextId = 10;
  let ui: UiStateDto = { ...DEFAULT_UI_STATE };
  const settings: SettingsDto = { embedEnabled: false, licenseNoticeAck: true, uiLang: 'vi', sampleText: 'Phở bò Hà Nội', projectViDefault: false, keepActiveOnExit: false, rescanOnLaunch: true };

  const lib = createMockLibraryApi(rows, { latencyMs: o.latencyMs });
  const delay = async () => { if (o.latencyMs) await new Promise(r => setTimeout(r, o.latencyMs)); };
  const findRow = (k: FaceKeyDto) => { const r = byKey.get(keyOf(k)); if (!r) throw new CmdError('not_found', 'face not found'); return r; };

  return {
    ...lib,
    async queryFaces(q) {
      await delay();
      const t = q.text ? fold(q.text) : '';
      const col = q.collectionId != null ? collections.get(q.collectionId) : null;
      let hits = rows.filter(r =>
        (!t || fold(r.family).includes(t) || fold(r.fullName ?? '').includes(t)) &&
        (!q.sourceIds?.length || q.sourceIds.includes(sourceOf(r))) &&
        (!q.favoritesOnly || r.isFavorite) &&
        (q.collectionId == null || !!col?.faces.has(keyOf(r))) &&
        (!q.tagIds?.length || q.tagIds.every(id => tags.get(id)?.faces.has(keyOf(r)))) &&
        (q.vi !== 'full' || r.viMissing === 0) && (q.vi !== 'supported' || r.supportsVi));
      hits = hits.sort((a, b) => fold(a.family).localeCompare(fold(b.family)) || (a.weightClass ?? 0) - (b.weightClass ?? 0) || +a.isItalic - +b.isItalic);
      const offset = q.offset ?? 0;
      const limit = Math.min(q.limit || 100, 1000);
      return { total: hits.length, offset, items: hits.slice(offset, offset + limit).map(r => ({ ...r })) };
    },
    previewUrl: (row, { text, px }) => svgPreview(text, px, familyOf.get(row.hashHex) ?? ''),
    fontUrl: () => null,

    listSources: () => ok(sources.map(s => ({ ...s, faceCount: countOf(s.id) }))),
    async addSource(path) {
      const hit = sources.find(s => s.path && (path === s.path || path.startsWith(`${s.path}/`) || s.path.startsWith(`${path}/`)));
      if (hit) throw new CmdError('nested_source', 'nested source', { existingId: hit.id });
      const s: SourceDto = { id: nextId++, kind: 'folder', name: path.split('/').pop() || path, path, enabled: true, available: true, faceCount: 0, lastScan: null, scanning: false };
      sources = [...sources, s];
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
    async deleteCollection(id) { collections.delete(id); },
    async addToCollection(id, faces) { const c = collections.get(id); if (!c) throw new CmdError('not_found', 'collection'); faces.forEach(f => c.faces.add(keyOf(f))); },
    async removeFromCollection(id, faces) { collections.get(id)?.faces && faces.forEach(f => collections.get(id)!.faces.delete(keyOf(f))); },
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
        row, summary: `${row.family} ${row.subfamily ?? ''}`.trim(), source: 'axes', hasImage: false,
        file: { format: 'ttf', container: null, outline: 'glyf', numGlyphs: 1200 + row.faceId * 7, sizeBytes: row.fileSize, path: row.path },
        metadata: {
          license: 'SIL Open Font License 1.1', licenseUrl: 'https://openfontlicense.org', designer: 'Studio Mock', manufacturer: 'Mock Foundry',
          version: 'Version 2.001', copyright: `© 2024 The ${row.family} Project Authors`, vendorUrl: null,
          variableAxes: [], namedInstances: [], scripts: ['latn', 'vietnamese'], opszRange: null, features: { gsub: ['ccmp', 'liga'], gpos: ['kern', 'mark'] },
        },
        vietnamese: { status: row.viStatus, supports: row.supportsVi, missing: row.viMissing ? 'ẰẲẴẶẸẺẼỀỂỄỆỈỊỌỎỐỒỔỖỘỚỜỞỠỢỤỦỨỪỬỮỰỲỴỶỸ'.slice(0, row.viMissing).split('') : [], stackedMarksOverflow: false, stackStyle: null },
      });
    },
    getSettings: () => ok(settings),
    activationState: () => ok([...active].map<ActivationStateDto>(k => ({ hashHex: k.split(':')[0], state: 'active', scope: 'user', status: 'ok', reasons: ['manual'], activePath: null, conflictPs: null, lastError: null }))),
    async activationActivate(faces) { faces.forEach(f => active.add(keyOf(f))); },
    async activationDeactivate(faces) { faces.forEach(f => active.delete(keyOf(f))); },
  };
}
