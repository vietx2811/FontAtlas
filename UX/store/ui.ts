// UI state của app (Plan 6b §3). Lọc không chạy ở client: view/tag/query/viOnly → GridQuery → query_faces.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api, FaceKeyDto, UiStateDto } from '../api';
import type { ComboRoleDto, OwnerDto } from '../bindings';
import { DEFAULT_UI_STATE } from '../api';
import type { GridQuery, FaceQuery } from '../features/library-grid/types';

export const PERSIST_DEBOUNCE_MS = 400;
export const RECENT_MAX = 200;
export const RECENT_DEBOUNCE_MS = 1000;
import { DEFAULT_SORT, SORT_KEY, defaultDesc, effectiveSort, isDefaultSort, parseSort, serializeSort, type SortField, type SortPrefs } from './sortPrefs';
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

export type LibraryFilters = Pick<FaceQuery, 'variable' | 'italic' | 'rawAxisRanges' | 'vi' | 'license' | 'sourceIds' | 'favoritesOnly' | 'classes' | 'subclasses' | 'weightClass' | 'widthClass' | 'varAxisMatch' | 'langs' | 'langMode' | 'tagIds'>;
export type DeskState = { caseId: number; comboId: number; role: ComboRoleDto; trial: FaceKeyDto | null };
export type FamilyScope = { name: string; faces: FaceKeyDto[] };

export interface UiState {
  filters: LibraryFilters;
  familyScope: FamilyScope | null;
  view: View;
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
  /** "Có sẵn trên máy": file giống hệt do bên khác cung cấp (hashHex → ai và ở đâu). Font Atlas không bật gì cho chúng; không tính vào "Đang bật". */
  available: ReadonlyMap<string, { owner: OwnerDto; path: string }>;
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
  /** Pairing Desk đang mở (Case + Combo + vai đang chọn + ứng viên đang thử); null = không mở. Loại trừ với `compare`/`pairThese`. */
  desk: DeskState | null;
  /** Desk đang mở lúc vào Compare: đóng Compare thì quay về đúng chỗ. */
  deskReturn: DeskState | null;
  /** Cách sắp xếp thư viện (toàn cục); lưu ở setting `ui.sort`. Gần đây / Đang bật khoá thứ tự. */
  sort: SortPrefs;
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
  /** Đóng và quên phiên so sánh (menu Sidebar: Đóng phiên so sánh). */
  closeCompareSession(): void;
  clearRecent(): void;
  setSortBy(by: SortField): void;
  setSortDesc(desc: boolean): void;
  resetSort(): void;
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
  openDesk(desk: DeskState): void;
  closeDesk(): void;
  closePairThese(): void;
  toggleSettings(): void;
  refreshActive(api: Api): Promise<void>;
  /** Tính lại "Có sẵn trên máy" từ hệ điều hành (mở app, đối soát, quay lại cửa sổ, sau mỗi lần bật/tắt). */
  refreshAvailable(api: Api): Promise<void>;
  bumpLibrary(reloadGrid?: boolean): void;
}

export const faceKey = (k: FaceKeyDto): string => `${k.hashHex}:${k.faceIndex}`;
export const parseFaceKey = (id: string): FaceKeyDto => {
  const i = id.lastIndexOf(':');
  return { hashHex: id.slice(0, i), faceIndex: Number(id.slice(i + 1)) };
};

/** Metadata dùng cho discovery/stats; không phụ thuộc membership của family/Gần đây/Đang bật. */
export function toMetadataQuery(s: Pick<UiState, 'view' | 'query' | 'viOnly'> & Partial<Pick<UiState, 'filters'>>): GridQuery {
  const q: GridQuery = { ...s.filters };
  if (s.view === 'star') q.favoritesOnly = true;
  else if (s.view.startsWith('col:')) q.collectionId = Number(s.view.slice(4));
  else if (s.view.startsWith('src:')) q.sourceIds = [Number(s.view.slice(4))];
  if (s.query.trim()) q.text = s.query.trim();
  if (s.viOnly && !q.vi) q.vi = 'full';
  // Chỉ báo ngôn ngữ trên thẻ theo ngôn ngữ đang lọc (đầu tiên); không lọc thì mặc định của backend (vie).
  if (q.langs?.length && !q.badgeLang) q.badgeLang = q.langs[0].code;
  return q;
}

/** Gần đây/Đang bật/family giới hạn library bằng faceKeys; discovery dùng toMetadataQuery. */
export function toGridQuery(s: Pick<UiState, 'view' | 'query' | 'viOnly' | 'recentViews' | 'active'> & Partial<Pick<UiState, 'filters' | 'familyScope' | 'sort'>>): GridQuery {
  const q = toMetadataQuery(s);
  // Sắp xếp của người dùng; 'match' chỉ khi đang có chữ tìm. Gần đây / Đang bật ghi đè bên dưới (thứ tự đưa vào).
  // Khớp chỉ khi đang tìm (mặc định lúc tìm); Tên/Ngày/Weight tường minh xếp thuần theo trường. Match không có chiều.
  const so = s.sort && effectiveSort(s.sort, !!s.query.trim());
  if (so && so.by === 'match') q.sort = 'match';
  else if (so && (so.explicit || !isDefaultSort(so))) { q.sort = so.by; q.descending = so.desc; }
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

export const dropTags = (f: LibraryFilters): LibraryFilters => { const { tagIds: _, ...rest } = f; return rest; };

/** Thẻ duy nhất đang lọc (chế độ "xem một thẻ" của thanh bên); nhiều thẻ hoặc không có → null. */
export const singleTagId = (f: LibraryFilters): number | null => (f.tagIds?.length === 1 ? f.tagIds[0] : null);

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
    filters: {}, familyScope: null, view: 'all', query: '', viOnly: false, sample: DEFAULT_SAMPLE_TEXT, sort: { ...DEFAULT_SORT },
    selectedId: null, style: null, panelTab: 'spec', active: new Map<string, readonly number[]>(), activeScope: new Map<string, 'session' | 'persistent'>(), available: new Map<string, { owner: OwnerDto; path: string }>(), recentViews: [], addFolderRequest: 0, settingsOpen: false, compare: null, compareSession: null, pairThese: null, desk: null, deskReturn: null, libraryRev: 0, gridRev: 0, ...initial,

    setFilters: filters => set({ filters, viOnly: filters.vi === 'full' }),
    openFamily: familyScope => set(s => ({ filters: dropTags(s.filters), familyScope, view: 'all', query: '', selectedId: null, style: null, panelTab: 'spec', settingsOpen: false, compare: null, pairThese: null, desk: null })),
    setView: v => set({ view: v, familyScope: null, settingsOpen: false, compare: null, pairThese: null, desk: null }),
    // Mục "Thẻ" ở thanh bên đặt tagIds = [id] (thay mọi thẻ đang chọn); bấm lại thẻ đang là thẻ duy nhất thì bỏ.
    setTag: t => set(s => ({ filters: t === null ? dropTags(s.filters) : { ...s.filters, tagIds: [Number(t)] }, familyScope: null, settingsOpen: false, compare: null, pairThese: null, desk: null })),
    // Xoá hết chữ tìm: Khớp không còn nghĩa → về mặc định (không để sort treo ở Khớp).
    setQuery: q => set(s => ({ query: q, familyScope: null, ...(!q.trim() && s.sort.by === 'match' ? { sort: { ...DEFAULT_SORT } } : {}) })),
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
      set({ compare: f, compareSession: { faces: f, pinned: [], overlayOn: false, overlayIds: [] }, pairThese: null, desk: null, deskReturn: get().desk, settingsOpen: false });
    },
    setSortBy: by => set({ sort: { by, desc: defaultDesc(by), explicit: true } }),
    setSortDesc: desc => set(s => ({ sort: { ...s.sort, desc, explicit: true } })),
    resetSort: () => set({ sort: { ...DEFAULT_SORT } }),
    closeCompareSession: () => set({ compare: null, compareSession: null }),
    clearRecent: () => set({ recentViews: [] }),
    closeCompare: () => set(s => ({ compare: null, desk: s.deskReturn, deskReturn: null })),
    resumeCompare: () => set(s => (s.compareSession ? { compare: s.compareSession.faces, pairThese: null, desk: null, settingsOpen: false } : s)),
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
    openPairThese: faces => { if (faces.length >= 2) set({ pairThese: faces.slice(0, 4), compare: null, desk: null, settingsOpen: false }); },
    closePairThese: () => set({ pairThese: null }),
    openDesk: desk => set({ desk, compare: null, pairThese: null, settingsOpen: false }),
    closeDesk: () => set({ desk: null, deskReturn: null }),
    toggleSettings: () => set(s => ({ settingsOpen: !s.settingsOpen })),
    requestAddFolder: () => set(s => ({ addFolderRequest: s.addFolderRequest + 1 })),
    toggleInfo: () => set(s => ({ infoExpanded: !s.infoExpanded })),
    bumpLibrary: (reloadGrid = false) => set(s => ({ libraryRev: s.libraryRev + 1, gridRev: reloadGrid ? s.gridRev + 1 : s.gridRev })),

    async refreshActive(api) {
      const st = await api.activationState();
      const on = st.filter(x => x.state === 'active');
      set({ active: new Map(on.map(x => [x.hashHex, x.faces] as const)), activeScope: new Map(on.map(x => [x.hashHex, x.scope] as const)) });
    },

    async refreshAvailable(api) {
      const list = await api.activationAvailability();
      set({ available: new Map(list.map(a => [a.hashHex, { owner: a.owner, path: a.path }] as const)) });
    },

    async hydrate(api) {
      stop?.();
      set(fromDto(await api.getUiState()));
      try {
        const parsed = JSON.parse((await api.getSetting(RECENT_KEY)) ?? '[]');
        set({ recentViews: Array.isArray(parsed) ? parsed.filter((r: RecentView) => typeof r?.face === 'string' && typeof r?.at === 'number').slice(0, RECENT_MAX) : [] });
      } catch { set({ recentViews: [] }); }
      try { set({ sort: parseSort(await api.getSetting(SORT_KEY)) }); } catch { set({ sort: { ...DEFAULT_SORT } }); }
      let sortWritten = serializeSort(get().sort);
      let sortTimer: ReturnType<typeof setTimeout> | undefined;
      // Ghi ngay phần còn treo khi dừng / đóng cửa sổ; `sortWritten` chỉ cập nhật sau khi ghi thành công (lỗi thì lần đổi sau thử lại).
      const flushSort = () => {
        clearTimeout(sortTimer); sortTimer = undefined;
        const now = serializeSort(get().sort);
        if (now === sortWritten) return;
        api.setSetting(SORT_KEY, now).then(() => { sortWritten = now; }, e => console.warn('set_setting failed', e));
      };
      window.addEventListener('pagehide', flushSort);
      let recentWritten = JSON.stringify(get().recentViews);
      let recentTimer: ReturnType<typeof setTimeout> | undefined;
      let written = JSON.stringify(toDto(get()));
      let timer: ReturnType<typeof setTimeout> | undefined;
      const unsub = store.subscribe(() => {
        const sjson = serializeSort(get().sort);
        if (sjson !== sortWritten) {
          clearTimeout(sortTimer);
          sortTimer = setTimeout(flushSort, PERSIST_DEBOUNCE_MS);
        }
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
      stop = () => { unsub(); clearTimeout(timer); clearTimeout(recentTimer); window.removeEventListener('pagehide', flushSort); flushSort(); };
    },
  }));
  return store;
}

export const useUiStore = createUiStore();
