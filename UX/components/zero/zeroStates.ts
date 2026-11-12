import type { SourceDto } from '../../api';
import type { AccessProblem } from '../../store/scan';
import type { View } from '../../store/ui';
import { t } from '../../i18n';

export type EmptyAction = { id: 'add-folder' | 'rescan' | 'clear-filters' | 'go-all' | 'grant'; label: string };
export type EmptyKind =
  | 'no-sources' | 'scanning' | 'sources-empty' | 'no-match' | 'no-match-vi' | 'favorites-empty' | 'recent-empty' | 'active-empty'
  | 'project-empty' | 'tag-empty' | 'source-unplugged' | 'source-empty' | 'source-access' | 'source-cloud';
export type Empty = { kind: EmptyKind; glyph: string; text: string; action?: EmptyAction; /** Hành động phụ (ví dụ Thử lại bên cạnh Cấp quyền). */ secondary?: EmptyAction };
export type EmptyCtx = { view: View; tag: string | null; query: string; viOnly: boolean; sources: SourceDto[]; filtered?: boolean; familyName?: string; scanning: boolean; access: Record<number, AccessProblem>; /** Đếm trực tiếp trong lúc quét, theo nguồn. */ cloud: Record<number, number> };

/** Thư viện (lưới) trống: ít chữ (một dòng, một hành động), mỗi tình huống một chữ minh hoạ nhỏ. */
/** Số file placeholder đám mây của một nguồn: đếm trực tiếp nếu đang quét, nếu không thì theo lần quét cuối. */
/**
 * Gợi ý cho nguồn nằm trên ổ cloud: tỉ lệ font còn là placeholder (số thật từ lần quét gần nhất) và cách giữ file trên máy.
 * null khi không phải nguồn cloud, chưa có font nào cần tải, hoặc nguồn rỗng.
 */
export function cloudHint(s: SourceDto): string | null {
  if (!s.cloudProvider || s.cloudOnlyCount <= 0 || s.faceCount <= 0) return null;
  const pct = Math.min(100, Math.round((s.cloudOnlyCount / s.faceCount) * 100));
  return t('cloud.sourceHint', { pct: pct === 0 ? '<1' : String(pct), action: keepActionOf(s.cloudProvider) });
}

/**
 * Tên mục menu Finder giữ thư mục luôn nằm trên máy, theo từng dịch vụ (tên do dịch vụ đặt, nên giữ nguyên tiếng Anh).
 * Font Atlas KHÔNG tự đặt cờ này: đó là quyết định về dung lượng ổ của người dùng, và đặt nó là ghi vào thư mục của họ.
 */
export function keepActionOf(provider: string | null): string {
  switch (provider) {
    case 'one_drive': return 'Always Keep on This Device';
    case 'google_drive': return 'Available offline';
    case 'dropbox': return 'Make available offline';
    case 'box': return 'Make Available Offline';
    case 'icloud': return 'Keep Downloaded';
    default: return 'Always keep on this device'; // dịch vụ File Provider khác: tên chung
  }
}

export const notDownloadedOf = (s: SourceDto, cloud: Record<number, number> = {}): number => cloud[s.id] ?? s.lastScan?.notDownloaded ?? 0;

export function libraryEmpty(c: EmptyCtx): Empty {
  const blocked = (id: number) => id in c.access;
  const access = (): Empty => ({ kind: 'source-access', glyph: 'ụ', text: t('zero.access'), action: { id: 'grant', label: t('zero.grant') }, secondary: { id: 'rescan', label: t('zero.retry') } });
  if (c.view.startsWith('src:') && blocked(Number(c.view.slice(4)))) return access();
  if (c.view === 'all' && c.sources.length > 0 && c.sources.every(x => blocked(x.id) && x.faceCount === 0)) return access();
  const cloud = (): Empty => ({ kind: 'source-cloud', glyph: 'ụ', text: t('zero.cloud'), action: { id: 'rescan', label: t('zero.rescan') } });
  if (c.scanning) return { kind: 'scanning', glyph: 'A', text: t('zero.scanning') };
  if (c.view.startsWith('src:')) {
    const s = c.sources.find(x => `src:${x.id}` === c.view);
    if (s && s.available && s.faceCount === 0 && notDownloadedOf(s, c.cloud) > 0) return cloud();
    if (s && !s.available) return { kind: 'source-unplugged', glyph: 'ụ', text: t('zero.unplugged'), action: { id: 'rescan', label: t('zero.rescanPlugged') } };
    if (s?.faceCount === 0) return { kind: 'source-empty', glyph: 'a', text: t('zero.srcEmpty'), action: { id: 'rescan', label: t('zero.rescan') } };
  }
  if (c.sources.length === 0) return { kind: 'no-sources', glyph: 'A', text: t('zero.noSources'), action: { id: 'add-folder', label: t('zero.addFolder') } };
  if (c.sources.length > 0 && c.sources.every(x => x.faceCount === 0 && notDownloadedOf(x, c.cloud) > 0)) return cloud();
  if (c.sources.every(s => s.faceCount === 0)) return { kind: 'sources-empty', glyph: 'A', text: t('zero.sourcesEmpty'), action: { id: 'rescan', label: t('zero.rescan') } };
  const q = c.query.trim();
  if (q) return { kind: 'no-match', glyph: 'ở', text: t('zero.noMatchQ', { q }), action: { id: 'clear-filters', label: t('zero.clearFilters') } };
  if (c.viOnly) return { kind: 'no-match-vi', glyph: 'Ặ', text: t('zero.noMatchVi'), action: { id: 'clear-filters', label: t('zero.clearVi') } };
  if (c.filtered) return { kind: 'no-match', glyph: 'ở', text: c.familyName ? t('zero.noMatchFam', { name: c.familyName }) : t('zero.noMatch'), action: { id: 'clear-filters', label: t('zero.clearFilters') } };
  if (c.tag !== null) return { kind: 'tag-empty', glyph: 'a', text: t('zero.tagEmpty'), action: { id: 'clear-filters', label: t('zero.clearTag') } };
  if (c.view === 'star') return { kind: 'favorites-empty', glyph: 'ẫ', text: t('zero.favEmpty') };
  if (c.view === 'recent') return { kind: 'recent-empty', glyph: 'ơ', text: t('zero.recentEmpty') };
  if (c.view === 'active') return { kind: 'active-empty', glyph: 'Ư', text: t('zero.activeEmpty') };
  if (c.view.startsWith('col:')) return { kind: 'project-empty', glyph: 'đ', text: t('zero.projectEmpty'), action: { id: 'go-all', label: t('zero.addFonts') } };
  return { kind: 'sources-empty', glyph: 'A', text: t('zero.sourcesEmpty'), action: { id: 'rescan', label: t('zero.rescan') } };
}
