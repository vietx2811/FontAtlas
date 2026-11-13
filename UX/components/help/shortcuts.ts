// Danh sách phím tắt THẬT của app (mỗi mục trỏ tới nơi đăng ký phím trong code) — dùng cho bảng "Phím tắt & tính năng chính".
// Khi thêm/đổi phím ở nơi đăng ký, sửa ở đây; test `shortcuts.test.ts` giữ danh sách không rỗng và mọi khoá i18n tồn tại.
import { useSyncExternalStore } from 'react';
import type { I18nKey } from '../../i18n';

export type ShortcutItem = { id: string; keys: readonly string[]; title: I18nKey; desc: I18nKey; where: string; /** Thẻ không có phím: bấm để mở Cài đặt ở nhóm này. */ openSettings?: string };
export const SHORTCUTS: readonly ShortcutItem[] = [
  { id: 'search', keys: ['⌘', 'K'], title: 'tips.search', desc: 'tips.search.d', where: 'main/SearchField.tsx' },
  { id: 'advanced', keys: ['⌥', '⌘', 'F'], title: 'tips.advanced', desc: 'tips.advanced.d', where: 'filter/AdvancedFilters.tsx' },
  { id: 'view', keys: ['⌘', 'J'], title: 'tips.view', desc: 'tips.view.d', where: 'main/ViewOptions.tsx' },
  { id: 'selectAll', keys: ['⌘', 'A'], title: 'tips.selectAll', desc: 'tips.selectAll.d', where: 'main/useLibrarySelection.ts' },
  { id: 'activate', keys: ['⇧', '⌘', 'A'], title: 'tips.activate', desc: 'tips.activate.d', where: 'main/TopBarActions.tsx (ActivationShortcut)' },
  { id: 'addto', keys: ['⌥', '⌘', 'P'], title: 'dnd.addTo', desc: 'dnd.addTo.tip', where: 'main/Library.tsx' },
  { id: 'pair', keys: ['⇧', 'P'], title: 'tips.pair', desc: 'tips.pair.d', where: 'main/TopBarActions.tsx' },
  { id: 'compare', keys: ['⇧', 'C'], title: 'tips.compare', desc: 'tips.compare.d', where: 'main/TopBarActions.tsx' },
  { id: 'desk', keys: ['[', ']'], title: 'tips.desk', desc: 'tips.desk.d', where: 'desk/PairingDesk.tsx' },
  { id: 'auto', keys: [], title: 'tips.auto', desc: 'tips.auto.d', where: 'settings/AutoActivation.tsx', openSettings: 'auto' },
  { id: 'settings', keys: ['⌘', ','], title: 'tips.settings', desc: 'tips.settings.d', where: 'settings/SettingsShortcut.tsx' },
];

// Trạng thái "bảng phím tắt mở lại từ ngoài onboarding" (phím ?).
let open = false;
const subs = new Set<() => void>();
export const setShortcutsHelpOpen = (v: boolean): void => { if (open !== v) { open = v; subs.forEach(f => f()); } };
export const useShortcutsHelpOpen = (): boolean => useSyncExternalStore(f => { subs.add(f); return () => { subs.delete(f); }; }, () => open, () => false);
