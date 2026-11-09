// UI state của app (Plan 6b §3). Lọc không chạy ở client: view/tag/query/viOnly → GridQuery → query_faces.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api, FaceKeyDto, UiStateDto } from '../api';
import { DEFAULT_UI_STATE } from '../api';
import type { GridQuery, FaceQuery } from '../features/library-grid/types';

export const PERSIST_DEBOUNCE_MS = 400;
export const RECENT_MAX = 200;
export const RECENT_DEBOUNCE_MS = 1000;
export const RECENT_KEY = 'ui.recent_views'; // 6a: ^ui\.[a-z0-9_.-]{1,60}$ (không chữ hoa)
const FACE_KEYS_MAX = 2000;
export const SAMPLE_MIN = 18;
export const SAMPLE_MAX = 72;
export const COMPARE_MAX = 8;
export const COMPARE_MIN = 2;
export const OVERLAY_MAX = 4;
export const DEFAULT_SAMPLE_TEXT = 'Phở bò Hà Nội';

export type View = 'all' | 'star' | 'recent' | 'active' | 'dups' | `col:${string}` | `src:${string}`;
export type RecentView = { face: string; at: number };
export type PanelTab = 'spec' | 'vi' | 'anat' | 'sim' | 'pair';
export type SectionKey = 'col' | 'src' | 'tag';
/** Phiên so sánh sống ngoài màn Compare (rời trang không mất): tập font, các cột pin (theo thứ tự pin), và overlay. */
export type CompareSession = { faces: FaceKeyDto[]; pinned: string[]; overlayOn: boolean; overlayIds: string[] };

export type LibraryFilters = Pick<FaceQuery, 'variable' | 'italic' | 'rawAxisRanges' | 'vi' | 'license' | 'sourceIds' | 'favoritesOnly' | 'classes' | 'subclasses'>;
export type FamilyScope = { name: string; faces: FaceKeyDto[] };

export interface UiState {
  filters: LibraryFilters;
  familyScope: FamilyScope | null;
  view: View;
  tag: string | null; // id thẻ (số, dạng chuỗi)
  query: string;
  viOnly: boolean;
  layout: 'grid' | 'list';
  sample: string;
  sampleSize: number;
  selectedId: string | null; // `${hashHex}:${faceIndex}`
  style: string | null;
  panelTab: PanelTab;
  sidebarCollapsed: boolean;
  openSections: Record<SectionKey, boolean>;
  infoExpanded: boolean;
  /** Các file đang bật: hashHex → các faceIndex của file (activation_state); nạp bằng refreshActive. */
  active: ReadonlyMap<string, readonly number[]>;
  /** Phạm vi bật của từng file đang bật (hashHex → session | persistent). */
  activeScope: ReadonlyMap<string, 'session' | 'persistent'>;
  /** Tăng mỗi lần có nơi nào đó (trạng thái trống) muốn mở hộp chọn thư mục; Sidebar lắng nghe. */
  addFolderRequest: number;
  /** Màn Cài đặt (S7) đang mở. */
  settingsOpen: boolean;
  /** Face đang so sánh (2–8); null = đang ở thư viện. */
  compare: FaceKeyDto[] | null;
  /** Phiên so sánh gần nhất; còn sau khi rời Compare (sidebar "Đang so sánh") cho tới khi mở phiên mới. */
  compareSession: CompareSession | null;
  /** Font đang ghép bằng "Pair these" (2–4); null = không mở. Loại trừ với `compare`. */
  pairThese: FaceKeyDto[] | null;
  /** Mới nhất trước, ≤ 200; lưu ở setting `ui.recent_views`. */
  recentViews: readonly RecentView[];
  /** Tăng khi dữ liệu thư viện đổi từ nơi khác (thêm vào dự án, gắn thẻ…): thanh bên nạp lại số đếm/danh sách. */
  libraryRev: number;
  /** Tăng khi lưới phải nạp lại (bỏ yêu thích trong view Yêu thích, gỡ khỏi dự án…). */
  gridRev: number;
}

export interface UiActions {
  setFilters(filters: LibraryFilters): void;
  openFamily(scope: FamilyScope): void;
  setView(v: View): void;
  setTag(t: string | null): void;
  setQuery(q: string): void;
  setViOnly(on: boolean): void;
  setLayout(l: 'grid' | 'list'): void;
  setSample(s: string): void;
  setSampleSize(n: number): void;
  select(id: string | null): void;
  setStyle(s: string | null): void;
  setPanelTab(t: PanelTab): void;
  toggleSidebar(): void;
  toggleSection(k: SectionKey): void;
  toggleInfo(): void;
  /** Đọc get_ui_state, rồi ghi ngược (debounce) mỗi khi một trường bền đổi. */
  hydrate(api: Api): Promise<void>;
  requestAddFolder(): void;
  setSettingsOpen(on: boolean): void;
  openCompare(faces: FaceKeyDto[]): void;
  closeCompare(): void;
  /** Quay lại phiên so sánh đang giữ (sidebar). */
  resumeCompare(): void;
  addCompareFace(f: FaceKeyDto): boolean;
  removeCompareFace(key: string): void;
  /** Gỡ các face khỏi phiên (không đọc được / thêm thất bại); không áp mức tối thiểu 2. */
  pruneCompareFaces(keys: string[]): void;
  toggleComparePin(key: string): void;
  setCompareOverlay(on: boolean): void;
  toggleCompareOverlayFace(key: string): void;
  openPairThese(faces: FaceKeyDto[]): void;
  closePairThese(): void;
  toggleSettings(): void;
  refreshActive(api: Api): Promise<void>;
  bumpLibrary(reloadGrid?: boolean): void;
}

export const faceKey = (k: FaceKeyDto): string => `${k.hashHex}:${k.faceIndex}`;
export const parseFaceKey = (id: string): FaceKeyDto => {
  const i = id.lastIndexOf(':');
  return { hashHex: id.slice(0, i), faceIndex: Number(id.slice(i + 1)) };
};

/** Metadata dùng cho discovery/stats; không phụ thuộc membership của family/Gần đây/Đang bật. */
export function toMetadataQuery(s: Pick<UiState, 'view' | 'tag' | 'query' | 'viOnly'> & Partial<Pick<UiState, 'filters'>>): GridQuery {
  const q: GridQuery = { ...s.filters };
  if (s.view === 'star') q.favoritesOnly = true;
  else if (s.view.startsWith('col:')) q.collectionId = Number(s.view.slice(4));
  else if (s.view.startsWith('src:')) q.sourceIds = [Number(s.view.slice(4))];
  if (s.tag !== null) q.tagIds = [Number(s.tag)];
  if (s.query.trim()) q.text = s.query.trim();
  if (s.viOnly && !q.vi) q.vi = 'full';
  return q;
}

/** Gần đây/Đang bật/family giới hạn library bằng faceKeys; discovery dùng toMetadataQuery. */
export function toGridQuery(s: Pick<UiState, 'view' | 'tag' | 'query' | 'viOnly' | 'recentViews' | 'active'> & Partial<Pick<UiState, 'filters' | 'familyScope'>>): GridQuery {
  const q = toMetadataQuery(s);
  if (s.view === 'recent') {
    q.faceKeys = s.recentViews.slice(0, FACE_KEYS_MAX).map(r => parseFaceKey(r.face));
    q.sort = 'match';
  } else if (s.view === 'active') {
    q.faceKeys = [...s.active].flatMap(([hashHex, faces]) => faces.map(faceIndex => ({ hashHex, faceIndex }))).slice(0, FACE_KEYS_MAX);
    q.sort = 'match';
  }
  if (s.familyScope) q.faceKeys = s.familyScope.faces;
  return q;
}

const SECTIONS: SectionKey[] = ['col', 'src', 'tag'];
const toDto = (s: UiState): UiStateDto => ({
  sidebarCollapsed: s.sidebarCollapsed,
  openSections: SECTIONS.filter(k => s.openSections[k]),
  layout: s.layout,
  sampleSize: s.sampleSize,
  infoExpanded: s.infoExpanded,
});
const fromDto = (d: UiStateDto): Pick<UiState, 'sidebarCollapsed' | 'openSections' | 'layout' | 'sampleSize' | 'infoExpanded'> => ({
  sidebarCollapsed: d.sidebarCollapsed,
  openSections: { col: d.openSections.includes('col'), src: d.openSections.includes('src'), tag: d.openSections.includes('tag') },
  layout: d.layout === 'list' ? 'list' : 'grid',
  sampleSize: Math.min(SAMPLE_MAX, Math.max(SAMPLE_MIN, d.sampleSize)),
  infoExpanded: d.infoExpanded,
});

/** Overlay đang bật mà việc bỏ cột làm cạn danh sách chọn → chọn lại mốc + font kế tiếp (chỉ khi do bỏ cột; tự bỏ chọn hết thì giữ nguyên). */
const reseedOverlay = (on: boolean, ids: string[], faces: FaceKeyDto[]): string[] => (on && ids.length === 0 ? faces.slice(0, 2).map(faceKey) : ids);

export type UiStore = UseBoundStore<StoreApi<UiState & UiActions>>;

export function createUiStore(): UiStore {
  const initial = fromDto(DEFAULT_UI_STATE);
  let stop: (() => void) | undefined;
  const store: UiStore = create<UiState & UiActions>()((set, get) => ({
    filters: {}, familyScope: null, view: 'all', tag: null, query: '', viOnly: false, sample: DEFAULT_SAMPLE_TEXT,
    selectedId: null, style: null, panelTab: 'spec', active: new Map<string, readonly number[]>(), activeScope: new Map<string, 'session' | 'persistent'>(), recentViews: [], addFolderRequest: 0, settingsOpen: false, compare: null, compareSession: null, pairThese: null, libraryRev: 0, gridRev: 0, ...initial,

    setFilters: filters => set({ filters, viOnly: filters.vi === 'full' }),
    openFamily: familyScope => set({ familyScope, view: 'all', tag: null, query: '', selectedId: null, style: null, panelTab: 'spec', settingsOpen: false, compare: null, pairThese: null }),
    setView: v => set({ view: v, tag: null, familyScope: null, settingsOpen: false, compare: null, pairThese: null }),
    setTag: t => set({ tag: t, familyScope: null, settingsOpen: false, compare: null, pairThese: null }),
    setQuery: q => set({ query: q, familyScope: null }),
    setViOnly: on => set(s => { const { vi: _vi, ...rest } = s.filters; return { viOnly: on, filters: on ? { ...rest, vi: 'full' } : rest }; }),
    setLayout: l => set({ layout: l }),
    setSample: s => set({ sample: s }),
    setSampleSize: n => set({ sampleSize: Math.min(SAMPLE_MAX, Math.max(SAMPLE_MIN, Math.round(n))) }),
    select: id => set(s => {
      if (s.selectedId === id) return s;
      if (id === null) return { selectedId: null, style: null };
      const recentViews = [{ face: id, at: Date.now() }, ...s.recentViews.filter(r => r.face !== id)].slice(0, RECENT_MAX);
      return { selectedId: id, style: null, recentViews };
    }),
    setStyle: s => set({ style: s }),
    setPanelTab: t => set({ panelTab: t }),
    toggleSidebar: () => set(s => ({ sidebarCollapsed: !s.sidebarCollapsed })),
    toggleSection: k => set(s => ({ openSections: { ...s.openSections, [k]: !s.openSections[k] } })),
    setSettingsOpen: on => set({ settingsOpen: on }),
    openCompare: faces => {
      if (faces.length < COMPARE_MIN) return;
      const f = faces.slice(0, COMPARE_MAX);
      set({ compare: f, compareSession: { faces: f, pinned: [], overlayOn: false, overlayIds: [] }, pairThese: null, settingsOpen: false });
    },
    closeCompare: () => set({ compare: null }),
    resumeCompare: () => set(s => (s.compareSession ? { compare: s.compareSession.faces, pairThese: null, settingsOpen: false } : s)),
    addCompareFace: f => {
      const s = get(); const cs = s.compareSession;
      if (!cs || cs.faces.length >= COMPARE_MAX || cs.faces.some(x => faceKey(x) === faceKey(f))) return false;
      const faces = [...cs.faces, f];
      set({ compareSession: { ...cs, faces }, compare: s.compare ? faces : s.compare });
      return true;
    },
    removeCompareFace: key => set(s => {
      const cs = s.compareSession;
      if (!cs || cs.faces.length <= COMPARE_MIN || !cs.faces.some(x => faceKey(x) === key)) return s;
      const faces = cs.faces.filter(x => faceKey(x) !== key);
      const overlayIds = reseedOverlay(cs.overlayOn, cs.overlayIds.filter(k => k !== key), faces);
      return { compareSession: { ...cs, faces, pinned: cs.pinned.filter(k => k !== key), overlayIds }, compare: s.compare ? faces : s.compare };
    }),
    pruneCompareFaces: keys => set(s => {
      const cs = s.compareSession;
      if (!cs || !cs.faces.some(x => keys.includes(faceKey(x)))) return s;
      const faces = cs.faces.filter(x => !keys.includes(faceKey(x)));
      const overlayIds = reseedOverlay(cs.overlayOn, cs.overlayIds.filter(k => !keys.includes(k)), faces);
      return { compareSession: { ...cs, faces, pinned: cs.pinned.filter(k => !keys.includes(k)), overlayIds }, compare: s.compare ? faces : s.compare };
    }),
    toggleComparePin: key => set(s => {
      const cs = s.compareSession;
      if (!cs || !cs.faces.some(x => faceKey(x) === key)) return s;
      return { compareSession: { ...cs, pinned: cs.pinned.includes(key) ? cs.pinned.filter(k => k !== key) : [...cs.pinned, key] } };
    }),
    setCompareOverlay: on => set(s => {
      const cs = s.compareSession;
      if (!cs) return s;
      // Bật lần đầu chưa chọn font nào: chọn sẵn mốc + font kế tiếp để có cái xem ngay.
      const overlayIds = on && cs.overlayIds.length === 0 ? cs.faces.slice(0, 2).map(faceKey) : cs.overlayIds;
      return { compareSession: { ...cs, overlayOn: on, overlayIds } };
    }),
    toggleCompareOverlayFace: key => set(s => {
      const cs = s.compareSession;
      if (!cs || !cs.faces.some(x => faceKey(x) === key)) return s;
      if (cs.overlayIds.includes(key)) return { compareSession: { ...cs, overlayIds: cs.overlayIds.filter(k => k !== key) } };
      if (cs.overlayIds.length >= OVERLAY_MAX) return s;
      return { compareSession: { ...cs, overlayIds: [...cs.overlayIds, key] } };
    }),
    openPairThese: faces => { if (faces.length >= 2) set({ pairThese: faces.slice(0, 4), compare: null, settingsOpen: false }); },
    closePairThese: () => set({ pairThese: null }),
    toggleSettings: () => set(s => ({ settingsOpen: !s.settingsOpen })),
    requestAddFolder: () => set(s => ({ addFolderRequest: s.addFolderRequest + 1 })),
    toggleInfo: () => set(s => ({ infoExpanded: !s.infoExpanded })),
    bumpLibrary: (reloadGrid = false) => set(s => ({ libraryRev: s.libraryRev + 1, gridRev: reloadGrid ? s.gridRev + 1 : s.gridRev })),

    async refreshActive(api) {
      const st = await api.activationState();
      const on = st.filter(x => x.state === 'active');
      set({ active: new Map(on.map(x => [x.hashHex, x.faces] as const)), activeScope: new Map(on.map(x => [x.hashHex, x.scope] as const)) });
    },

    async hydrate(api) {
      stop?.();
      set(fromDto(await api.getUiState()));
      try {
        const parsed = JSON.parse((await api.getSetting(RECENT_KEY)) ?? '[]');
        set({ recentViews: Array.isArray(parsed) ? parsed.filter((r: RecentView) => typeof r?.face === 'string' && typeof r?.at === 'number').slice(0, RECENT_MAX) : [] });
      } catch { set({ recentViews: [] }); }
      let recentWritten = JSON.stringify(get().recentViews);
      let recentTimer: ReturnType<typeof setTimeout> | undefined;
      let written = JSON.stringify(toDto(get()));
      let timer: ReturnType<typeof setTimeout> | undefined;
      const unsub = store.subscribe(() => {
        const rjson = JSON.stringify(get().recentViews);
        if (rjson !== recentWritten) {
          clearTimeout(recentTimer);
          recentTimer = setTimeout(() => {
            const now = JSON.stringify(get().recentViews);
            if (now === recentWritten) return;
            recentWritten = now;
            api.setSetting(RECENT_KEY, now).catch(e => console.warn('set_setting failed', e));
          }, RECENT_DEBOUNCE_MS);
        }
        clearTimeout(timer);
        timer = setTimeout(() => {
          const dto = toDto(get());
          const json = JSON.stringify(dto);
          if (json === written) return;
          written = json;
          api.setUiState(dto).catch(e => console.warn('set_ui_state failed', e));
        }, PERSIST_DEBOUNCE_MS);
      });
      stop = () => { unsub(); clearTimeout(timer); clearTimeout(recentTimer); };
    },
  }));
  return store;
}

export const useUiStore = createUiStore();
