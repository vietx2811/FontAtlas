// Hợp đồng IPC (Plan 6 cài bằng Tauri) + mock cho test và harness.
import { PX_BUCKETS, type FacePage, type FaceQuery, type FaceRow, type PxBucket } from './types';

export type PreviewInfo = { rendererVersion: string; generation: number; buckets: number[] };

export interface LibraryApi {
  queryFaces(q: FaceQuery): Promise<FacePage>;
  previewInfo(): Promise<PreviewInfo>;
  bumpGeneration(): Promise<number>;
  previewUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>, o: { text: string; px: PxBucket; generation: number; rendererVersion: string }): string;
  fontUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string | null;
}

/** Lệnh Tauri cho Plan 6 (khoá đối số camelCase). */
export interface TauriCommandMap {
  query_faces: { args: { q: FaceQuery }; result: FacePage };
  preview_info: { args: Record<string, never>; result: PreviewInfo };
  preview_bump_generation: { args: Record<string, never>; result: number };
  preview_prewarm: { args: { items: Array<{ hashHex: string; faceIndex: number; px: number; text: string; generation: number }> }; result: null };
}

/** toUrl: tên file → URL của scheme (Plan 6 dùng convertFileSrc(name, 'fontatlas-preview')). */
export function makePreviewUrl(toUrl: (fileName: string) => string): LibraryApi['previewUrl'] {
  return ({ hashHex, faceIndex }, { text, px, generation, rendererVersion }) =>
    `${toUrl(`${hashHex}_${faceIndex}_${px}.png`)}?t=${encodeURIComponent(text.normalize('NFC').trim())}&g=${generation}&r=${rendererVersion}`;
}

/** Font chỉ phục vụ face 0 (FontFace không chọn được face trong TTC). */
export function makeFontUrl(toUrl: (fileName: string) => string): LibraryApi['fontUrl'] {
  return ({ hashHex, faceIndex }) => (faceIndex > 0 ? null : toUrl(`${hashHex}_${faceIndex}.font`));
}

export type MockApi = LibraryApi & { calls: FaceQuery[]; currentGeneration(): number };

const fold = (s: string) => s.normalize('NFD').replace(/\p{Mn}/gu, '').replace(/đ/gi, 'd').toLowerCase();

export function createMockLibraryApi(
  rows: FaceRow[],
  o: { latencyMs?: number; toUrl?: (n: string) => string; fail?: (q: FaceQuery) => Error | null } = {},
): MockApi {
  const calls: FaceQuery[] = [];
  let generation = 0; // sống qua nhiều lần mount, như backend
  const sorted = [...rows].sort((a, b) => fold(a.family).localeCompare(fold(b.family)) || a.faceId - b.faceId);
  return {
    calls,
    currentGeneration: () => generation,
    async queryFaces(q) {
      calls.push(q);
      if (o.latencyMs) await new Promise(r => setTimeout(r, o.latencyMs));
      const err = o.fail?.(q);
      if (err) throw err;
      const t = q.text ? fold(q.text) : '';
      const hits = t ? sorted.filter(r => fold(r.family).includes(t)) : sorted;
      const offset = q.offset ?? 0;
      const limit = Math.min(q.limit || 100, 1000);
      return { total: hits.length, offset, items: hits.slice(offset, offset + limit) };
    },
    async previewInfo() {
      return { rendererVersion: 'mock', generation, buckets: [...PX_BUCKETS] };
    },
    async bumpGeneration() {
      return ++generation;
    },
    previewUrl: makePreviewUrl(o.toUrl ?? (n => `/preview/${n}`)),
    fontUrl: makeFontUrl(n => `/font/${n}`),
  };
}

export function makeRow(i: number, over: Partial<FaceRow> = {}): FaceRow {
  return {
    faceId: i + 1, hashHex: i.toString(16).padStart(64, '0'), faceIndex: 0,
    family: `Family ${String(i).padStart(5, '0')}`, subfamily: null, fullName: null,
    weightClass: null, widthClass: null, isItalic: false, isVariable: false,
    viStatus: 'full', supportsVi: true, viMissing: 0, licenseClass: 'ofl',
    path: `/fonts/${i}.ttf`, fileSize: 200_000, missing: false, onCloudOnly: false, isFavorite: false, addedAt: 0, langBadge: null,
    releaseYear: null, releaseYearSource: null, fileCreatedAt: null, fileCreatedSource: null,
    ...over,
  };
}
