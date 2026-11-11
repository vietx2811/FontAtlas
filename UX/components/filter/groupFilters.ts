import type { SourceDto } from '../../api';
import type { LibraryFilters } from '../../store/ui';
import { t, type I18nKey } from '../../i18n';
import { filterChips, type Chip } from '../main/FilterChips';

export type GroupId = 'class' | 'style' | 'vi' | 'lang' | 'library' | 'tags' | 'license' | 'axes';
export type Group = { id: GroupId; label: string; chips: Chip[]; /** Ghi chú ngắn sau giá trị trong tóm tắt (vd cách ghép ngôn ngữ "một trong"). */ suffix?: string; remove(f: LibraryFilters): LibraryFilters };

const ORDER: ReadonlyArray<[GroupId, I18nKey]> = [
  ['class', 'fgroup.class'], ['style', 'fgroup.style'], ['vi', 'fgroup.vi'], ['lang', 'fgroup.lang'], ['library', 'fgroup.library'], ['tags', 'fgroup.tags'], ['license', 'fgroup.license'], ['axes', 'fgroup.axes'],
];

const groupOf = (chipId: string): GroupId => {
  const head = chipId.split(':')[0];
  if (head === 'class' || head === 'subclass') return 'class'; // gồm cả công tắc class-hybrid / class-confident
  if (head === 'variable' || head === 'italic') return 'style';
  if (head === 'vi') return 'vi';
  if (head === 'lang') return 'lang';
  if (head === 'favorites' || head === 'source') return 'library';
  if (head === 'tag') return 'tags';
  if (head === 'license') return 'license';
  return 'axes';
};

/** Bộ lọc → nhóm theo thứ tự cố định (chỉ nhóm có giá trị). Dùng chung cho tóm tắt, "Đang lọc", Backspace và toast. Thuần, không đọc store. */
export function groupFilters(f: LibraryFilters, sources: SourceDto[] = [], view = 'all', tags: ReadonlyArray<{ id: number; name: string }> = []): Group[] {
  const chips = filterChips(f, sources, view, tags);
  const out: Group[] = [];
  for (const [id, key] of ORDER) {
    const mine = chips.filter(c => groupOf(c.id) === id);
    if (!mine.length) continue;
    // Gỡ cả nhóm = áp lần lượt remove của từng chip; mỗi remove tự chỉ đụng khoá của nó.
    const suffix = id === 'lang' && (f.langs?.length ?? 0) > 1 ? t(f.langMode === 'any' ? 'fgroup.lang.any' : 'fgroup.lang.all') : undefined;
    out.push({ id, label: t(key), chips: mine, suffix, remove: x => mine.reduce((acc, c) => c.remove(acc), x) });
  }
  return out;
}

/**
 * Hoàn tác MỘT thao tác gỡ nhóm: đặt lại đúng các khoá mà thao tác đó đã đổi (`before` → `after`) lên bộ lọc HIỆN TẠI,
 * giữ nguyên mọi thay đổi người dùng làm sau đó (khác với pop ngăn xếp, vốn khôi phục trạng thái của thao tác gần nhất).
 */
export function restoreChanged(before: LibraryFilters, after: LibraryFilters, current: LibraryFilters): LibraryFilters {
  const out = { ...current } as Record<string, unknown>;
  const b = before as Record<string, unknown>; const a = after as Record<string, unknown>;
  for (const k of new Set([...Object.keys(b), ...Object.keys(a)])) {
    if (JSON.stringify(b[k]) === JSON.stringify(a[k])) continue;
    if (b[k] === undefined) delete out[k]; else out[k] = b[k];
  }
  return out as LibraryFilters;
}
