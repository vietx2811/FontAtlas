// Backend giả cho dev (trình duyệt/`cargo tauri dev` không có 6a) và Vitest. Hành vi bám bảng lệnh 6a.
import { composeCompare } from './compare';
import { downloadBlob } from './download';
import { cssSize, type FaceRow, type PxBucket } from '../features/library-grid/types';
import { createMockLibraryApi } from '../features/library-grid/api';
import type { Api } from './index';
import { createActivationSim } from './mockActivation';
import { createModelSim } from './mockModel';
import { classBins, matchesWeight, matchesWidth, measuredHistogram } from './mockFilters';
import { VI_TOTAL, axesFor, MEASURED_AXES, MOCK_UNITS, pairFor, comboAssessment, similarFor, viMissingFor } from './mockAnalysis';
import {
  CmdError, DEFAULT_UI_STATE, type ActivationStateDto, type CollectionDto, type FaceDetailDto, type FaceKeyDto,
  type FamilyGroupDto, type FamilyStyleDto, type SettingsDto, type SourceDto, type SystemTagGroupDto, type TagDto, type UiStateDto,
} from './types';

const WEIGHTS: Array<[string, number]> = [['Thin', 100], ['ExtraLight', 200], ['Light', 300], ['Regular', 400], ['Medium', 500], ['SemiBold', 600], ['Bold', 700], ['ExtraBold', 800], ['Black', 900]];
// [family, nguồn, kiểu: 'ri' = đủ đứng+nghiêng 9 weight, 'r4' = Regular/Bold + nghiêng, '1' = một kiểu, viMissing]
const FAMILIES: Array<[string, number, 'ri' | 'r4' | '1', number]> = [
  ['Public Sans', 1, 'ri', 0], ['Roboto', 1, 'r4', 0], ['Inter', 1, 'ri', 0], ['Be Vietnam Pro', 1, 'ri', 0],
  ['Playfair Display', 1, 'r4', 0], ['Source Serif 4', 1, 'r4', 0], ['Fraunces', 2, 'r4', 3], ['DM Serif Display', 2, '1', 0],
  ['Bebas Neue', 2, '1', 12], ['Pacifico', 2, '1', 21], ['Lobster', 2, '1', 18], ['Oswald', 2, 'r4', 5],
  ['IBM Plex Mono', 2, 'r4', 0], ['JetBrains Mono', 1, 'r4', 0], ['Cormorant', 2, 'r4', 2], ['Abril Fatface', 3, '1', 9],
  // Taxonomy v2: Slab, Handwriting, Display (có class phụ) để UI thấy đủ class mới.
  ['Roboto Slab', 1, '1', 0], ['Caveat', 1, '1', 4], ['Monoton', 1, '1', 7],
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
      fileSize: 180_000 + n * 1000, missing: false, onCloudOnly: false, isFavorite: false, addedAt: n, langBadge: null,
      // R2 giả: 1/5 face không rõ năm, 1/7 không đọc được ngày file; mọi ngày file là `disk` trừ nguồn 3 (giả cloud)
      releaseYear: n % 5 === 0 ? null : 1995 + ((n * 7) % 30), releaseYearSource: n % 5 === 0 ? null : (['copyright', 'head_created', 'head_modified'] as const)[n % 3],
      fileCreatedAt: n % 7 === 0 ? null : 1_600_000_000_000 + ((n * 37) % 100) * 86_400_000, fileCreatedSource: n % 7 === 0 ? null : src === 3 ? 'cloud_sync' : 'disk',
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

import { createMockCases } from './mockCases';
import { createMockSuggest } from './mockSuggest';
import { createMockAuto, type MockAutoOptions } from './mockAuto';
import { badgeOf, coverageFor, listFor, matchesLangs } from './mockLang';
import { createMockResolve } from './mockResolve';
import type { DuplicateGroupDto, DuplicateMemberDto, BootStatusDto, EmbedStatusDto, LegacyStateDto, ModelStatusDto, OnboardingDto, ScanEvent, AxisHistogramDto, NameClashDto } from './types';
export type MockOptions = { auto?: MockAutoOptions; latencyMs?: number; tickMs?: number; /** Lần chạy đầu: chưa nguồn, chưa xác nhận giấy phép, chưa onboarding. */ firstRun?: boolean; legacy?: boolean; initial?: { model?: Partial<ModelStatusDto>; embed?: Partial<EmbedStatusDto> } };

export function createMockApi(o: MockOptions = {}): Api {
  const auto = createMockAuto(o.auto);
  const rows = mockRows();
  const byKey = new Map(rows.map(r => [keyOf(r), r]));
  const familyOf = new Map(rows.map(r => [r.hashHex, r.family]));
  const classOver = new Map<string, { class: string; subclass: string | null }>();
  const effClass = (r: FaceRow) => classOver.get(keyOf(r)) ?? mockAutoClass(r.family);
  // Class phụ chỉ có ở nhãn tự động (ghi đè của người dùng không có class phụ), như backend.
  const secClass = (r: FaceRow) => (classOver.has(keyOf(r)) ? null : mockSecondary(r.family));
  // Ghi đè của người dùng = tin cậy tuyệt đối; còn lại dùng độ tin cậy tự động. Mock không có class phụ nên `includeSecondary` không đổi kết quả.
  const mockConfidence = (r: FaceRow) => (classOver.has(keyOf(r)) ? 1 : mockAutoClass(r.family).class === 'uncertain' ? 0 : mockAutoClass(r.family).confidence);
  const sourceOf = (r: FaceRow) => Number(/src(\d)/.exec(r.path ?? '')?.[1] ?? 1);

  const cloudFetched = new Set<number>();
  const cloudRuns = new Map<number, boolean>();
  let sources: SourceDto[] = o.firstRun ? [] : [
    { id: 1, kind: 'folder', name: 'Fonts', path: '/Users/me/Fonts/src1', enabled: true, available: true, cloudProvider: null, cloudOnlyCount: 0, faceCount: 0, lastScan: null, scanning: false },
    { id: 2, kind: 'folder', name: 'Dự án khách', path: '/Users/me/Library/CloudStorage/OneDrive-Demo/src2', enabled: true, available: true, cloudProvider: 'one_drive', cloudOnlyCount: 0, faceCount: 0, lastScan: null, scanning: false },
    { id: 3, kind: 'folder', name: 'Ổ ngoài', path: '/Volumes/Backup/src3', enabled: true, available: false, cloudProvider: null, cloudOnlyCount: 0, faceCount: 0, lastScan: null, scanning: false },
  ];
  // Nguồn thứ hai có vài file còn là placeholder đám mây (để QA và chụp ảnh).
  const lastScanOf = (id: number) => (id === 2 ? { status: 'ok' as const, seen: 32, unchanged: 20, notDownloaded: 12, relinked: 0, analyzed: 0, failed: 0, missing: 0, unreadable: [] } : null);
  const countOf = (id: number) => rows.filter(r => sourceOf(r) === id).length;
  /** Mọi nhóm trùng của mock (chưa lọc, chưa trừ bản đã ẩn / nhóm bỏ qua). */
  const allDupGroups = async (): Promise<DuplicateGroupDto[]> => {
    const pick = (family: string, sub: string) => rows.find(r => r.family === family && r.subfamily === sub) ?? rows.find(r => r.family === family)!;
    const m = (r: FaceRow, path: string | null, o: Partial<DuplicateMemberDto> = {}): DuplicateMemberDto => ({ row: r, path, format: 'ttf', version: null, revision: null, modified: null, isNewest: false, ...o });
    const a = pick('Be Vietnam Pro', 'Regular'), p1 = pick('Playfair Display', 'Regular'), p2 = pick('Playfair Display', 'Bold');
    const r1 = pick('Roboto', 'Regular'), r2 = pick('Public Sans', 'Regular'), o1 = pick('Oswald', 'Regular');
    const i1 = pick('Inter', 'Regular'), s1 = pick('Source Serif 4', 'Regular');
    return [
      { kind: 'same_file', key: a.hashHex, newestKnown: false, faces: [m(a, a.path), m(a, (a.path ?? '/Fonts/x.ttf').replace('/Fonts/', '/Downloads/'))] },
      { kind: 'versions', key: 'PlayfairDisplay-Regular', newestKnown: true, faces: [m(p1, p1.path, { version: 'Version 1.203', revision: 1.203, modified: 1_600_000_000 }), m(p2, p2.path, { version: 'Version 1.204', revision: 1.204, modified: 1_650_000_000, isNewest: true })] },
      { kind: 'versions', key: 'Roboto-Regular', newestKnown: false, faces: [m(r1, r1.path), m(r2, `/Users/me/Fonts/src2/Roboto-Regular-copy.ttf`)] },
      { kind: 'formats', key: 'Oswald|Regular|400|false', newestKnown: false, faces: [m(o1, o1.path), m(o1, '/Volumes/Ổ ngoài/Oswald-Regular.otf', { format: 'sfnt' }), m(o1, '/Fonts/Oswald-Regular.woff2', { format: 'woff2', version: 'Version 4.1', revision: 4.1 })] },
      // Bản thua nằm ở ~/Library/Fonts (cần tick riêng) và /Library/Fonts (không được vào Thùng rác).
      { kind: 'same_file', key: i1.hashHex, newestKnown: false, faces: [m(i1, i1.path), m(i1, '/Users/me/Library/Fonts/Inter-Regular.ttf')] },
      { kind: 'same_file', key: s1.hashHex, newestKnown: false, faces: [m(s1, s1.path), m(s1, '/Library/Fonts/SourceSerif4-Regular.ttf')] },
    ];
  };
  const resolveMock = createMockResolve({ baseGroups: allDupGroups, sourceIds: () => sources.map(s => s.id) });

  const collections = new Map<number, CollectionDto & { faces: Set<string> }>();
  const tags = new Map<number, TagDto & { faces: Set<string> }>();
  let nextId = 10;
  let ui: UiStateDto = { ...DEFAULT_UI_STATE };
  const uiSettings = new Map<string, string>();
  const scanListeners = new Set<(e: ScanEvent) => void>();
  let scanning = false;
  let cacheBytes = 12 * 1024 * 1024;
  const mockNameClashes: NameClashDto[] = [];
  let onboarded = !o.firstRun;
  let legacy: LegacyStateDto = { found: !!o.legacy, preview: o.legacy ? { fonts: 1240, sources: 2, collections: 3, favorites: 17, tags: 9 } : null, imported: null, resolved: null, error: null };
  const legacyState = () => legacy;
  const boot = (): BootStatusDto => ({ phase: 'ready', storeError: null, libraryRecreated: false, userBackup: null, legacy });
  const settings: SettingsDto = { embedEnabled: false, licenseNoticeAck: !o.firstRun, uiLang: null, sampleText: 'Phở bò Hà Nội', projectViDefault: false, keepActiveOnExit: false, rescanOnLaunch: true, cloudPreviewDownload: true, cloudWatch: true };

  const simulateScan = (id: number) => {
    const t = o.tickMs ?? 400; const emit = (e: ScanEvent) => scanListeners.forEach(l => l(e));
    scanning = true;
    emit({ type: 'queued', sourceId: id });
    [0.25, 0.5, 0.75, 1].forEach((f, i) => setTimeout(() => {
      emit({ type: 'progress', sourceId: id, progress: { phase: 'analyzing', done: Math.round(f * 200), total: 200, facesWritten: Math.round(f * 180), notDownloaded: Math.round(f * 7) } });
      emit({ type: 'facesAdded', sourceId: id, count: 45 });
      if (f === 1) {
        emit({ type: 'finished', sourceId: id, stats: { status: 'ok', seen: 200, unchanged: 0, relinked: 0, analyzed: 195, failed: 3, missing: 0, unreadable: [], notDownloaded: 7 } });
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

  const caseApi = createMockCases(k => byKey.get(keyOf(k)));
  const suggestApi = createMockSuggest({ rows: () => rows, caseFaces: async id => (await caseApi.getCase(id)).faces.map(f => f.key) });
  return {
    ...lib,
    ...caseApi,
    ...suggestApi,
    async briefStarter(brief) {
      // cùng bảng ánh xạ với backend (services/brief.rs): tone → term (vùng chết 0.25, ràng buộc cứng từ 0.6), mục đích → term
      const TONE: Record<string, [string, string]> = { classic_modern: ['classic', 'modern'], warm_cool: ['warm', 'cool'], quiet_bold: ['delicate', 'strong'] };
      const USE: Record<string, string[]> = { editorial: ['editorial'], ui_web: ['readable_small', 'clean'], poster: ['display', 'heavy'], branding: [], packaging: [] };
      const ids: string[] = [];
      for (const t of brief.tones) if (Math.abs(t.value) >= 0.25) ids.push(TONE[t.pair][t.value < 0 ? 0 : 1]);
      for (const u of brief.uses) for (const id of USE[u]) if (!ids.includes(id)) ids.push(id);
      const notes: Array<{ code: string; detail: string | null }> = [];
      if (brief.uses.length === 1 && brief.uses[0] === 'branding') notes.push({ code: 'branding_class_only', detail: null });
      if (brief.uses.includes('packaging')) notes.push({ code: 'packaging_inferred', detail: 'display=display+heavy; body=readable_small+clean' });
      if (!ids.length && !brief.uses.includes('packaging')) notes.push({ code: 'no_terms', detail: null });
      const { items } = await this.queryFaces({ sort: 'family', limit: Infinity, vi: brief.needs.vi ? 'supported' : undefined, license: brief.needs.openLicense ? ['ofl', 'apache'] : undefined });
      const seen = new Set<string>();
      const anchors = items.filter(r => !seen.has(r.family) && seen.add(r.family)).slice(0, 3);
      if (anchors.length < 3) notes.push({ code: 'few_anchors', detail: String(anchors.length) });
      const usedBody = new Set<string>();
      const combos = anchors.flatMap((a, i) => {
        const body = items.find(r => r.family !== a.family && (r.weightClass ?? 400) <= 500 && !usedBody.has(r.family));
        if (!body) { notes.push({ code: 'no_body_pair', detail: a.family }); return []; }
        usedBody.add(body.family);
        return [{ heading: a, body, assessment: comboAssessment(a, body, i), rank: i + 1 }];
      });
      const query = { text: null, vi: brief.needs.vi ? 'supported' : null, license: brief.needs.openLicense ? ['ofl', 'apache'] : [], variable: brief.needs.variable ? true : null, italic: brief.needs.italic ? true : null, sourceIds: [], favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'match', descending: false, offset: 0, limit: 60, axisFilters: [], hard: brief.needs.vi ? ['vi_supported'] : [], rawAxisRanges: [], faceKeys: null, classes: [], subclasses: [], traits: [], includeSecondary: false, confidentOnly: false, minConfidence: null, langs: brief.needs.vi ? [{ code: 'vie', level: 'near' }] : [], langMode: 'all', badgeLang: null, weightClass: null, widthClass: null, varAxisMatch: { wght: true, wdth: true }, groupByFamily: false } as const;
      return {
        describe: { dictionaryVersion: 'mock', matched: ids.map(id => ({ id, text: id, group: 'mock', subjectivity: '', culture: '', rationaleVi: '', rationaleEn: '', explanationBanned: false })), filters: [], hard: [...query.hard], conflicts: [], unmappable: [], partial: [], unknownWords: [] },
        query: { ...query } as never, displayQuery: { ...query, weightClass: [600, 900] } as never, bodyQuery: { ...query, weightClass: [400, 500] } as never,
        anchors, combos, notes, ai: null,
      };
    },
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
        (!q.license?.length || q.license.includes(r.licenseClass)) &&
        (q.variable == null || r.isVariable === q.variable) && (q.italic == null || r.isItalic === q.italic) &&
        (!q.classes?.length || q.classes.includes(effClass(r).class) || (!!q.includeSecondary && q.classes.includes(secClass(r)?.class ?? '')))
        && (!q.subclasses?.length || q.subclasses.includes(effClass(r).subclass ?? '') || (!!q.includeSecondary && q.subclasses.includes(secClass(r)?.subclass ?? ''))) &&
        (!(q.confidentOnly || q.minConfidence != null) || mockConfidence(r) >= Math.max(q.confidentOnly ? 0.88 : 0, q.minConfidence ?? 0)) &&
        (q.includeMissing || !r.missing) &&
        (!q.weightClass || matchesWeight(r, q.weightClass, q.varAxisMatch?.wght ?? true)) &&
        (!q.widthClass || matchesWidth(r, q.widthClass, q.varAxisMatch?.wdth ?? true)) &&
        (!q.rawAxisRanges?.length || q.rawAxisRanges.every(f => { const a = axesFor(r).find(a => a.key === f.axis); return a && a.quality !== 'failed' && a.rawValue !== null && a.rawValue >= f.range[0] && a.rawValue <= f.range[1]; })) &&
        (q.collectionId == null || !!col?.faces.has(keyOf(r))) &&
        (!q.tagIds?.length || q.tagIds.every(id => tags.get(id)?.faces.has(keyOf(r)))) &&
        (!q.langs?.length || matchesLangs(r, q.langs, q.langMode)) &&
        (q.vi !== 'full' || r.viMissing === 0) && (q.vi !== 'supported' || r.supportsVi));
      if (q.sort === 'match' && wanted) hits = hits.sort((a, b) => wanted.get(keyOf(a))! - wanted.get(keyOf(b))!);
      else if (q.sort === 'released' || q.sort === 'file_created') {
        // không rõ luôn cuối bất kể chiều, rồi cùng bậc theo Family → Weight → nghiêng (như store)
        const val = (r: FaceRow) => (q.sort === 'released' ? r.releaseYear : r.fileCreatedAt);
        const byFamily = (a: FaceRow, b: FaceRow) => fold(a.family).localeCompare(fold(b.family)) || (a.weightClass ?? 0) - (b.weightClass ?? 0) || +a.isItalic - +b.isItalic;
        hits = hits.sort((a, b) => { const x = val(a), y = val(b); if (x == null || y == null) return x == null && y == null ? byFamily(a, b) : x == null ? 1 : -1; return (x - y) * (q.descending ? -1 : 1) || byFamily(a, b); });
      }
      else hits = hits.sort((a, b) => fold(a.family).localeCompare(fold(b.family)) || (a.weightClass ?? 0) - (b.weightClass ?? 0) || +a.isItalic - +b.isItalic);
      if (q.groupByFamily) {
        // mỗi family liền nhau, nhóm theo thứ tự xuất hiện đầu tiên (= cực trị theo sort/chiều), trong nhóm Weight → thẳng trước nghiêng
        const order: string[] = []; const by = new Map<string, FaceRow[]>();
        for (const r of hits) { const k = fold(r.family); if (!by.has(k)) { by.set(k, []); order.push(k); } by.get(k)!.push(r); }
        hits = order.flatMap(k => by.get(k)!.sort((a, b) => (a.weightClass ?? 0) - (b.weightClass ?? 0) || +a.isItalic - +b.isItalic));
      }
      const offset = q.offset ?? 0;
      const limit = q.limit === Infinity ? Infinity : Math.min(q.limit || 100, 1000); // Infinity chỉ dùng nội bộ cho thống kê
      return { total: hits.length, offset, items: hits.slice(offset, offset + limit).map(r => ({ ...r, langBadge: badgeOf(r, q.badgeLang ?? 'vie') })) };
    },
    previewUrl: (row, { text, px }) => {
      const r = byKey.get(keyOf(row));
      return svgPreview(text, px, familyOf.get(row.hashHex) ?? '', r?.weightClass ?? 400, !!r?.isItalic);
    },
    fontUrl: () => null,

    // nguồn cloud trong mock: 9/10 font còn là placeholder (để QA và chụp ảnh gợi ý "Always keep on this device")
    listSources: () => ok(sources.map(s => ({ ...s, faceCount: countOf(s.id), cloudOnlyCount: s.cloudProvider && !cloudFetched.has(s.id) ? Math.round(countOf(s.id) * 0.9) : 0, lastScan: s.lastScan ?? lastScanOf(s.id) }))),
    async addSource(path) {
      const hit = sources.find(s => s.path && (path === s.path || path.startsWith(`${s.path}/`) || s.path.startsWith(`${path}/`)));
      if (hit) throw new CmdError('nested_source', 'nested source', { existingId: hit.id, existingPath: hit.path });
      const s: SourceDto = { id: nextId++, kind: 'folder', name: path.split('/').pop() || path, path, enabled: true, available: true, cloudProvider: null, cloudOnlyCount: 0, faceCount: 0, lastScan: null, scanning: false };
      sources = [...sources, s];
      simulateScan(s.id);
      return ok(s);
    },
    async removeSourcePreview(id) {
      if (!sources.some(s => s.id === id)) throw new CmdError('not_found', 'source');
      const n = countOf(id);
      return { removedLocations: n, facesLeftWithoutLocation: n, casesAffected: 0, collectionsAffected: [...collections.values()].filter(c => rows.some(r => sourceOf(r) === id && c.faces.has(keyOf(r)))).length, tagsAffected: [...tags.values()].filter(t => rows.some(r => sourceOf(r) === id && t.faces.has(keyOf(r)))).length, activeFonts: 0 };
    },
    async removeSource(id) { const r = await this.removeSourcePreview(id); sources = sources.filter(s => s.id !== id); return r; },
    async rescanSources() {},
    async revealSource(id) {
      const s = sources.find(x => x.id === id);
      if (!s) throw new CmdError('not_found', 'source');
      if (!s.available || !s.path) throw new CmdError('source_unavailable', 'source unavailable', { sourceId: id, name: s.name });
    },
    async sourceDownloadCloud(id, onProgress) {
      const s = sources.find(x => x.id === id);
      if (!s) throw new CmdError('not_found', 'source');
      if (!s.available) throw new CmdError('source_unavailable', 'source unavailable', { sourceId: id, name: s.name });
      if (cloudRuns.has(id)) throw new CmdError('busy', 'already downloading');
      const total = s.cloudProvider && !cloudFetched.has(id) ? Math.round(countOf(id) * 0.9) : 0;
      cloudRuns.set(id, false);
      let done = 0;
      onProgress?.({ done, total, failed: 0 });
      while (done < total && !cloudRuns.get(id)) { await Promise.resolve(); done += 1; onProgress?.({ done, total, failed: 0 }); }
      const cancelled = done < total;
      cloudRuns.delete(id);
      if (!cancelled && total > 0) cloudFetched.add(id);
      return { total, downloaded: done, failed: 0, cancelled, rescanQueued: done > 0, firstFailureCode: null };
    },
    async sourceDownloadCancel(id) { if (!cloudRuns.has(id)) return false; cloudRuns.set(id, true); return true; },

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
    systemTagCatalog: () => ok(MOCK_SYSTEM_TAGS),
    listTags: () => ok([...tags.values()].map(({ faces, ...t }) => ({ ...t, faceCount: faces.size }))),
    async tagsOf(k) { return ok([...tags.values()].filter(t => t.faces.has(keyOf(k))).map(({ faces, ...t }) => ({ ...t, faceCount: faces.size }))); },
    async createTag(name) { name = canonicalSystemTag(name); const id = nextId++; tags.set(id, { id, name, faceCount: 0, faces: new Set() }); return ok({ id, name, faceCount: 0 }); },
    async tagFaces(id, faces) { const t = tags.get(id); if (!t) throw new CmdError('not_found', 'tag'); faces.forEach(f => t.faces.add(keyOf(f))); },
    async renameTag(id, name) {
      const t = tags.get(id); if (!t) throw new CmdError('not_found', 'tag');
      const n = canonicalSystemTag(name.trim()); if (!fold(n)) throw new CmdError('invalid_arg', 'empty tag name');
      if ([...tags.values()].some(x => x.id !== id && fold(x.name) === fold(n))) throw new CmdError('conflict', 'tag name exists');
      if (/^(style|mood|use):/.test(t.name) && !/^(style|mood|use):/.test(n)) throw new CmdError('invalid_arg', 'system tag');
      t.name = n; return ok({ id, name: n, faceCount: t.faces.size });
    },
    async deleteTag(id) { const t = tags.get(id); if (!t) throw new CmdError('not_found', 'tag'); tags.delete(id); return t.faces.size; },
    async untagFaces(id, faces) { const t = tags.get(id); if (t) faces.forEach(f => t.faces.delete(keyOf(f))); },
    async revealInFinder(k) { findRow(k); },
    async revealLocation(path) { if (!rows.some(r => r.path === path) && !path.includes('/')) throw new CmdError('not_found', 'path'); },
    // Mock: Latin + Latin mở rộng (U+0000–024F) và Việt (U+1E00–1EFF) có; còn lại (CJK…) thiếu.
    async missingChars(face, chars) { findRow(face); return chars.filter(g => g.trim() && [...g.normalize('NFC')].some(c => { const n = c.codePointAt(0)!; return !(n <= 0x24f || (n >= 0x300 && n <= 0x36f) || (n >= 0x1e00 && n <= 0x1eff)); })); },
    getUiState: () => ok(ui),
    async setUiState(s) { ui = { ...s }; },

    async compareFaces(faces) {
      const c = await composeCompare(this, faces);
      c.faces.forEach(f => {
        const sib = rows.filter(r => r.family === f.row.family);
        const weights = [...new Set(sib.map(r => r.weightClass ?? 400))].sort((a, b) => a - b);
        const serif = mockAutoClass(f.row.family).class === 'serif';
        f.extras = {
          extrasAvailable: true, vendorId: f.row.faceId % 3 === 0 ? null : 'MOCK', years: { created: 2016 + (f.row.faceId % 6), modified: 2021, copyright: 2016 + (f.row.faceId % 6) },
          otGroups: [{ group: 'figures', tags: ['lnum', 'onum'], sets: [] }, { group: 'ligatures', tags: ['liga'], sets: [] }, ...(serif ? [{ group: 'small_caps', tags: ['smcp'], sets: [] }, { group: 'stylistic_sets', tags: ['ss01', 'ss02'], sets: [{ tag: 'ss01', name: 'Alternate a' }, { tag: 'ss02', name: null }] }] : [])],
          kerning: { source: 'gpos', pairs: 4000 + f.row.faceId * 37, classPairs: 1200 + f.row.faceId * 11 },
          style: { italicKind: f.row.isItalic ? 'italic' : 'upright', italAxis: false, slntAxis: false, wghtRange: f.row.isVariable ? [400, 900] : null, wdthRange: null, opszRange: null },
          embedding: { fsType: 0, level: 'installable', flags: [] }, licenseKind: 'ofl',
        };
        f.family = { family: f.row.family, faces: sib.length, weights, widths: [5], hasItalic: sib.some(r => r.isItalic), hasOblique: false, hasOpsz: false, hasVariable: sib.some(r => r.isVariable) };
        f.licenseNote = { kind: 'ofl', description: 'SIL Open Font License 1.1', url: 'https://openfontlicense.org', embedding: f.extras.embedding };
        f.license = 'ofl';
        if (f.vi.stackedMarksTopEm != null) { f.vi.overCapPctEm = Math.max(0, Math.round((f.vi.stackedMarksTopEm - 0.7) * 1000) / 10); f.vi.overAscenderPctEm = f.vi.stackedMarksOverflow ? Math.max(1, f.vi.overCapPctEm - 30) : 0; }
      });
      return c;
    },
    async pairThese(faces, o = {}) {
      if (faces.length < 2 || faces.length > 4) throw new CmdError('invalid_arg', 'pair these needs 2–4 faces');
      const picked = faces.map(findRow);
      const all: Array<[FaceRow, FaceRow]> = picked.flatMap(h => picked.filter(b => b !== h && (!o.projectVi || b.viMissing === 0)).map(b => [h, b] as [FaceRow, FaceRow]));
      return all.slice(0, Math.min(3, Math.max(1, o.maxCombos ?? 3))).map(([h, b], i) => ({ heading: h, body: b, assessment: comboAssessment(h, b, i), rank: i + 1 }));
    },
    async savePng(png, name) { return downloadBlob(png, name); },
    async duplicateGroups(q = {}) {
      const keep = new Set((await this.queryFaces({ ...q, offset: 0, limit: 1000 })).items.map(keyOf));
      return (await resolveMock.liveGroups()).filter(g => g.faces.some(f => keep.has(keyOf(f.row))));
    },
    ...resolveMock.methods,
    ...({ __resolve: resolveMock } as object),
    async duplicateSummary(q = {}) {
      const g = await this.duplicateGroups(q);
      const n = (k: string) => g.filter(x => x.kind === k).length;
      return { sameFile: n('same_file'), versions: n('versions'), formats: n('formats'), total: g.length };
    },
    async activationActivateCase(caseId, o = {}) {
      const c = await this.getCase(caseId);
      const report = await this.activationActivate(c.faces.filter(f => !f.missing).map(f => f.key), o);
      return { report, skippedMissing: c.faces.filter(f => f.missing).map(f => f.key) };
    },
    async familyGroups(q = {}) {
      const { items } = await this.queryFaces({ ...q, groupByFamily: true, offset: 0, limit: Infinity });
      const totals = new Map<string, number>();
      for (const r of rows) if (!r.missing) totals.set(fold(r.family), (totals.get(fold(r.family)) ?? 0) + 1);
      const out: FamilyGroupDto[] = [];
      items.forEach((r, i) => {
        const last = out[out.length - 1];
        if (last && fold(last.family) === fold(r.family)) { last.count++; last.matchCount++; return; }
        out.push({ family: r.family, start: i, count: 1, matchCount: 1, styleTotal: totals.get(fold(r.family)) ?? 1 });
      });
      return out;
    },
    async classFacets(q = {}) {
      const { items } = await this.queryFaces({ ...q, classes: [], subclasses: [], offset: 0, limit: 1000 });
      const by = new Map<string, Map<string, number>>();
      // includeSecondary: face còn được đếm thêm ở class phụ của nó, nên tổng facet có thể lớn hơn số face duy nhất.
      const slots = (r: FaceRow) => [effClass(r), ...(q.includeSecondary && secClass(r) ? [secClass(r)!] : [])];
      for (const r of items) for (const c of slots(r)) { const m = by.get(c.class) ?? new Map(); if (c.subclass) m.set(c.subclass, (m.get(c.subclass) ?? 0) + 1); by.set(c.class, m); }
      const count = (cls: string) => items.filter(r => slots(r).some(c => c.class === cls)).length;
      return [...by].map(([cls, subs]) => ({ class: cls, count: count(cls), subclasses: [...subs].map(([subclass, n]) => ({ subclass, count: n })) }));
    },
    async setClassificationOverride(o) { findRow(o.face); classOver.set(keyOf(o.face), v2Override(o.class, o.subclass)); },
    async clearClassificationOverride(face) { classOver.delete(keyOf(face)); },
    async languageList(q = {}) { return listFor((await this.queryFaces({ ...q, langs: [], offset: 0, limit: Infinity })).items); },
    async langCoverage(faceId, code) { const r = rows.find(x => x.faceId === faceId); if (!r) throw new CmdError('not_found', 'face not found'); return coverageFor(r, code); },
    async axisStats(q = {}) {
      const { items } = await this.queryFaces({ ...q, rawAxisRanges: [], weightClass: undefined, widthClass: undefined, offset: 0, limit: Infinity });
      return MEASURED_AXES.map(axis => {
        const values = items.flatMap(r => { const a = axesFor(r).find(a => a.key === axis); return a?.rawValue != null && a.quality !== 'failed' ? [a.rawValue] : []; }).sort((a, b) => a - b);
        const quantile = (p: number) => { if (!values.length) return null; const at = (values.length - 1) * p; const i = Math.floor(at); return values[i] + (values[Math.ceil(at)] - values[i]) * (at - i); };
        return { axis, unit: MOCK_UNITS[axis], count: values.length, min: values[0] ?? null, max: values.at(-1) ?? null, p1: quantile(.01), p99: quantile(.99) };
      });
    },
    async axisHistogram(q = {}, bins = 0) {
      const nb = bins === 0 ? 24 : Math.min(64, Math.max(2, bins));
      const ranges = q.rawAxisRanges ?? [];
      // miền: bỏ MỌI bộ lọc trục (kể cả weight/width class); đếm: còn weight/width class, trừ khoảng của chính trục
      const domainRows = (await this.queryFaces({ ...q, rawAxisRanges: [], weightClass: undefined, widthClass: undefined, offset: 0, limit: Infinity })).items;
      const countRows = (await this.queryFaces({ ...q, rawAxisRanges: [], offset: 0, limit: Infinity })).items;
      const val = (r: FaceRow, axis: string) => { const a = axesFor(r).find(a => a.key === axis); return a?.rawValue != null && a.quality !== 'failed' ? a.rawValue : null; };
      const out: AxisHistogramDto[] = MEASURED_AXES.map(axis => {
        const others = ranges.filter(r => r.axis !== axis);
        const domain = domainRows.flatMap(r => { const v = val(r, axis); return v == null ? [] : [v]; });
        const counted = countRows.flatMap(r => { const v = val(r, axis); return v != null && others.every(f => { const x = val(r, f.axis); return x != null && x >= f.range[0] && x <= f.range[1]; }) ? [v] : []; });
        return measuredHistogram(axis, MOCK_UNITS[axis], domain, counted, nb);
      });
      const klass = async (axis: 'weight' | 'width') => {
        const own = axis === 'weight' ? 'weightClass' : 'widthClass';
        const { items } = await this.queryFaces({ ...q, [own]: undefined, offset: 0, limit: Infinity });
        const { bins: b, spanBins } = classBins(items, axis);
        out.push({ axis: `${axis}_class`, unit: 'class', count: b.reduce((x, y) => x + y, 0), lo: axis === 'weight' ? 100 : 1, hi: axis === 'weight' ? 900 : 9, bins: b, spanBins, empty: spanBins.every(n => n === 0) });
      };
      await klass('weight');
      await klass('width');
      return out;
    },
    async similarFamilies(face, opts = {}, q = {}) {
      const row = findRow(face);
      const { items } = await this.queryFaces({ ...q, offset: 0, limit: 1000 });
      const candidates = items.filter(r => (!opts.viOnly || r.viMissing === 0) && (!opts.sourceIds?.length || opts.sourceIds.includes(sourceOf(r))) && (!opts.weightRange || (r.weightClass != null && r.weightClass >= opts.weightRange[0] && r.weightClass <= opts.weightRange[1])));
      const ranked = similarFor(row, candidates, candidates.length).map(h => opts.useImage === false ? { ...h, source: 'axes' as const } : h);
      const groups = new Map<string, { family: string; representative: typeof ranked[number]; faces: FaceKeyDto[]; count: number }>();
      for (const hit of ranked) {
        const group = groups.get(hit.row.family);
        if (group) { group.faces.push(hit.face); group.count++; }
        else groups.set(hit.row.family, { family: hit.row.family, representative: hit, faces: [hit.face], count: 1 });
      }
      const offset = q.offset ?? 0;
      return { total: groups.size, offset, items: [...groups.values()].slice(offset, offset + (q.limit ?? 12)) };
    },

    async getFace(k, _lang): Promise<FaceDetailDto> {
      const row = findRow(k);
      return ok({
        row, summary: `${row.family} ${row.subfamily ?? ''}`.trim(), classification: /Serif|Didone|Fraunces|Cormorant|Abril|Playfair/.test(row.family) ? 'Serif' : /Mono/.test(row.family) ? 'Mono' : 'Sans', source: 'axes', hasImage: false,
        classificationInfo: mockClassification(row.family, classOver.get(keyOf(row))),
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
          markWeightRatio: { key: 'mark_weight_ratio', value: 0.82, rawValue: 0.82, unit: 'ratio', quality: 'measured', reason: null, method: 'outline', percentile: 55 },
        },
        axes: axesFor(row),
        similarTop: similarFor(row, rows, 3),
        extras: mockExtras(),
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
    async nameClashes() { return mockNameClashes; },
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
    async setWindowGlass() { return false; }, // trình duyệt/mock: không có kính native
    async legacyImport() { legacy = { ...legacyState(), imported: { fonts: 1240, sources: 2, collections: 3, recoveredCollections: 0, badTags: 0, skippedSources: 0 } }; return ok(legacy); },
    async legacyRetry() { return ok(legacyState()); },
    async scanStatus() { return ok({ running: scanning, paused: false, currentSourceId: null, queued: [] }); },
    async scanPause() {}, async scanResume() {}, async scanCancel() {},
    async scanSubscribe(l) { scanListeners.add(l); return () => { scanListeners.delete(l); }; },
    ...act,
    ...auto,
  };
}

/** Phân loại tự động giả cho mock (cố định theo họ font). */
/** Class phụ (hybrid) giả: chỉ hiển thị/tìm kiếm; lọc/nhóm luôn theo class chính. */
const MOCK_SECONDARY: Array<[RegExp, { class: string; subclass: string | null }]> = [[/Monoton/, { class: 'sans', subclass: 'geometric' }]];
export const mockSecondary = (family: string): { class: string; subclass: string | null } | null => MOCK_SECONDARY.find(([re]) => re.test(family))?.[1] ?? null;
const MOCK_AUTO: Array<[RegExp, string, string | null, number]> = [
  [/Roboto Slab/, 'slab', 'clarendon', 0.92],
  [/Caveat/, 'handwriting', 'marker', 0.9],
  [/Monoton/, 'display', 'inline_outline', 0.91],
  [/Pacifico|Lobster/, 'uncertain', null, 0.55],
  [/Didone|Playfair|Abril|Bodoni/, 'serif', 'didone', 0.94],
  [/Serif|Fraunces|Cormorant/, 'serif', 'garalde', 0.9],
  [/Mono/, 'mono', null, 0.95],
  [/Bebas/, 'unclassified', null, 0],
  [/Oswald/, 'sans', 'grotesque', 0.9],
];
const CLASS_LABEL: Record<string, string> = { serif: 'Serif', slab: 'Slab', sans: 'Sans', mono: 'Mono', script: 'Script', handwriting: 'Handwriting', display: 'Display', blackletter: 'Blackletter', symbol: 'Symbol', uncertain: 'Uncertain', unclassified: 'Chưa phân loại' };
/** Như backend: ghi đè nhãn v1 được nhận nhưng LƯU thành v2 (serif/slab → slab, handwritten/script cũ → handwriting, formal → formal_copperplate…). */
const V1_SUB: Record<string, string> = { humanist_oldstyle: 'venetian_oldstyle', old_style: 'garalde', formal: 'formal_copperplate', casual_brush: 'brush', handwritten: 'casual_print' };
export function v2Override(cls: string, sub: string | null): { class: string; subclass: string | null } {
  if (cls === 'serif/slab') return { class: 'slab', subclass: null };
  return { class: cls, subclass: sub ? V1_SUB[sub] ?? sub : null };
}
export function mockAutoClass(family: string): { class: string; subclass: string | null; confidence: number } {
  const hit = MOCK_AUTO.find(([re]) => re.test(family));
  return hit ? { class: hit[1], subclass: hit[2], confidence: hit[3] } : { class: 'sans', subclass: null, confidence: 0.9 };
}
/** Dữ liệu D2 giả cho mock (không có font thật để đọc). */
function mockExtras(): FaceDetailDto['extras'] {
  return {
    extrasAvailable: true, vendorId: 'MOCK', years: { created: 2017, modified: 2021, copyright: 2017 },
    otGroups: [{ group: 'figures', tags: ['lnum', 'onum'], sets: [] }],
    kerning: { source: 'gpos', pairs: 1200, classPairs: 800 },
    style: { italicKind: 'upright', italAxis: false, slntAxis: false, wghtRange: null, wdthRange: null, opszRange: null },
    embedding: { fsType: 0, level: 'installable', flags: [] }, licenseKind: 'ofl',
  };
}

function mockClassification(family: string, over?: { class: string; subclass: string | null }): FaceDetailDto['classificationInfo'] {
  const auto = mockAutoClass(family);
  const cls = over?.class ?? auto.class;
  return {
    class: cls, label: CLASS_LABEL[cls] ?? cls, subclass: over ? over.subclass : auto.subclass, confidence: over ? 1 : auto.confidence,
    candidate: cls === 'uncertain' ? 'script' : null,
    // Cố ý có giá trị lạ như dữ liệu thật (vote 'usage', nguồn và trait chưa dịch) để UI không giả định tập khoá đóng.
    signals: [{ source: 'measure', detail: 'serif_foot', vote: auto.class, weight: 0.6 }, ...(/Abril|Pacifico/.test(family) ? [{ source: 'future_source', detail: 'x', vote: 'usage', weight: 0.1 }] : [])],
    traits: /Bebas/.test(family) ? ['fat_face'] : /Abril/.test(family) ? ['Fat face', 'Quirky_new_trait'] : [],
    usage: auto.class === 'sans' || auto.class === 'serif' ? ['text'] : [], source: over ? 'override' : 'auto', autoClass: over ? auto.class : null, secondary: over ? null : mockSecondary(family),
  };
}

// Danh mục tag hệ thống giống backend (fontatlas-core/system_tags.rs): phong cách, mood, mục đích dùng.
const mkSys = (namespace: string, label: string, tags: [string, string][]): SystemTagGroupDto => ({ namespace, label, tags: tags.map(([key, l]) => ({ key, label: l, name: `${namespace}:${key}` })) });
export const MOCK_SYSTEM_TAGS: SystemTagGroupDto[] = [
  mkSys('style', 'Style', [['art_deco', 'Art Deco'], ['art_nouveau', 'Art Nouveau'], ['wood_type', 'Wood type'], ['western', 'Western'], ['retro_vintage', 'Retro / Vintage'], ['psychedelic', 'Psychedelic'], ['brutalist', 'Brutalist'], ['techno', 'Techno'], ['medieval', 'Medieval'], ['tuscan', 'Tuscan'], ['distressed', 'Distressed'], ['blobby', 'Blobby'], ['wacky', 'Wacky']]),
  mkSys('mood', 'Mood', [['business', 'Business'], ['calm', 'Calm'], ['elegant', 'Elegant'], ['playful', 'Playful'], ['friendly_cute', 'Friendly / Cute'], ['loud_bold', 'Loud / Bold'], ['vintage_retro', 'Vintage / Retro'], ['futuristic_tech', 'Futuristic / Tech'], ['rugged', 'Rugged'], ['artistic', 'Artistic'], ['formal', 'Formal'], ['handmade', 'Handmade']]),
  mkSys('use', 'Use', [['editorial', 'Editorial'], ['logo_branding', 'Logo / Branding'], ['packaging', 'Packaging'], ['signage', 'Signage']]),
];
/** Như backend `create_tag`: tiền tố dành riêng được chuẩn hoá và phải có trong danh mục; tên khác giữ nguyên. */
function canonicalSystemTag(raw: string): string {
  const name = raw.trim();
  const i = name.indexOf(':');
  if (i < 0) return name;
  const g = MOCK_SYSTEM_TAGS.find(x => x.namespace === name.slice(0, i).trim().toLowerCase());
  if (!g) return name;
  const t = g.tags.find(x => x.key === name.slice(i + 1).trim().toLowerCase());
  if (!t) throw new CmdError('invalid_arg', `tag hệ thống không có trong danh mục: ${name}`);
  return t.name;
}
