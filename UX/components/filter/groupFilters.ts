import type { SourceDto } from '../../api';
import type { LibraryFilters } from '../../store/ui';
import { t, type I18nKey } from '../../i18n';
import { filterChips, type Chip } from '../main/FilterChips';

export type GroupId = 'class' | 'style' | 'vi' | 'library' | 'license' | 'axes';
export type Group = { id: GroupId; label: string; chips: Chip[]; remove(f: LibraryFilters): LibraryFilters };

const ORDER: ReadonlyArray<[GroupId, I18nKey]> = [
  ['class', 'fgroup.class'], ['style', 'fgroup.style'], ['vi', 'fgroup.vi'], ['library', 'fgroup.library'], ['license', 'fgroup.license'], ['axes', 'fgroup.axes'],
];

const groupOf = (chipId: string): GroupId => {
  const head = chipId.split(':')[0];
  if (head === 'class' || head === 'subclass') return 'class';
  if (head === 'variable' || head === 'italic') return 'style';
  if (head === 'vi') return 'vi';
  if (head === 'favorites' || head === 'source') return 'library';
  if (head === 'license') return 'license';
  return 'axes';
};

/** Bộ lọc → nhóm theo thứ tự cố định (chỉ nhóm có giá trị). Dùng chung cho tóm tắt, "Đang lọc", Backspace và toast. Thuần, không đọc store. */
export function groupFilters(f: LibraryFilters, sources: SourceDto[] = [], view = 'all'): Group[] {
  const chips = filterChips(f, sources, view);
  const out: Group[] = [];
  for (const [id, key] of ORDER) {
    const mine = chips.filter(c => groupOf(c.id) === id);
    if (!mine.length) continue;
    // Gỡ cả nhóm = áp lần lượt remove của từng chip; mỗi remove tự chỉ đụng khoá của nó.
    out.push({ id, label: t(key), chips: mine, remove: x => mine.reduce((acc, c) => c.remove(acc), x) });
  }
  return out;
}
