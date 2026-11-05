import type { SourceDto } from '../../api';
import type { AccessProblem } from '../../store/scan';
import type { View } from '../../store/ui';

export type EmptyAction = { id: 'add-folder' | 'rescan' | 'clear-filters' | 'go-all' | 'grant'; label: string };
export type EmptyKind =
  | 'no-sources' | 'scanning' | 'sources-empty' | 'no-match' | 'no-match-vi' | 'favorites-empty' | 'recent-empty' | 'active-empty'
  | 'project-empty' | 'tag-empty' | 'source-unplugged' | 'source-empty' | 'source-access';
export type Empty = { kind: EmptyKind; glyph: string; text: string; action?: EmptyAction; /** Hành động phụ (ví dụ Thử lại bên cạnh Cấp quyền). */ secondary?: EmptyAction };
export type EmptyCtx = { view: View; tag: string | null; query: string; viOnly: boolean; sources: SourceDto[]; scanning: boolean; access: Record<number, AccessProblem> };

/** Thư viện (lưới) trống: ít chữ (một dòng, một hành động), mỗi tình huống một chữ minh hoạ nhỏ. */
export function libraryEmpty(c: EmptyCtx): Empty {
  const q = c.query.trim();
  if (q) return { kind: 'no-match', glyph: 'ở', text: `Không có font khớp “${q}”`, action: { id: 'clear-filters', label: 'Xoá bộ lọc' } };
  if (c.viOnly) return { kind: 'no-match-vi', glyph: 'Ặ', text: 'Không có font nào đủ dấu tiếng Việt', action: { id: 'clear-filters', label: 'Bỏ lọc Ặ vi' } };
  if (c.tag !== null) return { kind: 'tag-empty', glyph: 'a', text: 'Chưa có font gắn thẻ này', action: { id: 'clear-filters', label: 'Bỏ thẻ' } };
  if (c.view === 'star') return { kind: 'favorites-empty', glyph: 'ẫ', text: 'Chưa có font yêu thích. Bấm ngôi sao trên thẻ.' };
  if (c.view === 'recent') return { kind: 'recent-empty', glyph: 'ơ', text: 'Chưa mở font nào gần đây' };
  if (c.view === 'active') return { kind: 'active-empty', glyph: 'Ư', text: 'Chưa bật font nào. Dùng công tắc trong panel chi tiết.' };
  if (c.view.startsWith('col:')) return { kind: 'project-empty', glyph: 'đ', text: 'Dự án trống', action: { id: 'go-all', label: 'Thêm font' } };
  const blocked = (id: number) => id in c.access;
  const ACCESS: Empty = { kind: 'source-access', glyph: 'ụ', text: 'Cần quyền truy cập thư mục này', action: { id: 'grant', label: 'Cấp quyền' }, secondary: { id: 'rescan', label: 'Thử lại' } };
  if (c.view.startsWith('src:') && blocked(Number(c.view.slice(4)))) return ACCESS;
  if (c.view === 'all' && c.sources.length > 0 && c.sources.every(x => blocked(x.id) && x.faceCount === 0)) return ACCESS;
  if (c.view.startsWith('src:')) {
    const s = c.sources.find(x => `src:${x.id}` === c.view);
    if (s && !s.available) return { kind: 'source-unplugged', glyph: 'ụ', text: 'Ổ chưa kết nối. Font vẫn nằm trong thư viện.', action: { id: 'rescan', label: 'Quét lại khi cắm' } };
    return { kind: 'source-empty', glyph: 'a', text: 'Nguồn này chưa có font', action: { id: 'rescan', label: 'Quét lại' } };
  }
  if (c.sources.length === 0) return { kind: 'no-sources', glyph: 'A', text: 'Chưa có font. Kéo thư mục vào đây hoặc thêm thủ công.', action: { id: 'add-folder', label: 'Thêm thư mục font' } };
  if (c.scanning) return { kind: 'scanning', glyph: 'A', text: 'Đang quét, font sẽ hiện dần' };
  return { kind: 'sources-empty', glyph: 'A', text: 'Các nguồn chưa có font nào', action: { id: 'rescan', label: 'Quét lại' } };
}
