import type { MenuEntry } from '../ui/Menu';
import { t } from '../../i18n';

/** Mục Sidebar có menu chuột phải (spec library-view-options C6). Tất cả / Yêu thích / Trùng lặp không có menu. */
export type SidebarMenuKind = 'source' | 'collection' | 'tag' | 'recent' | 'active' | 'compare';

export type SidebarMenuCtx = {
  open(): void;
  /** Nguồn */
  sourceAvailable?: boolean; hasPath?: boolean; scanning?: boolean; reveal?(): void; rescan?(): void;
  /** Dự án */
  rename?(): void; deactivateAll?(): void; hasItems?: boolean; remove?(): void;
  /** Thẻ */
  filterByTag?(): void;
  /** Gần đây */
  recentCount?: number; clearRecent?(): void;
  /** Đang bật */
  activeCount?: number;
  /** Đang so sánh */
  closeSession?(): void;
};

const open = (c: SidebarMenuCtx): MenuEntry => ({ id: 'open', label: t('smenu.open'), onSelect: c.open });
const sep: MenuEntry = { type: 'sep' };

/** Hàm thuần: mô hình menu theo loại mục. Mục không dùng được vẫn hiện, xám, kèm lý do. */
export function buildSidebarMenu(kind: SidebarMenuKind, c: SidebarMenuCtx): MenuEntry[] {
  switch (kind) {
    case 'source': return [
      open(c),
      { id: 'reveal', label: t('smenu.reveal'), disabled: !c.sourceAvailable || !c.hasPath, reason: t('smenu.reveal.off'), onSelect: c.reveal },
      { id: 'rescan', label: t('smenu.rescan'), disabled: !!c.scanning || !c.sourceAvailable, reason: c.scanning ? t('smenu.rescan.busy') : t('smenu.reveal.off'), onSelect: c.rescan },
    ];
    case 'collection': return [
      open(c),
      { id: 'rename', label: t('smenu.rename'), hint: 'F2', onSelect: c.rename },
      { id: 'deactivate', label: t('smenu.deactivateAll'), disabled: !c.hasItems, reason: t('smenu.empty'), onSelect: c.deactivateAll },
      sep,
      { id: 'delete', label: t('smenu.deleteCollection'), danger: true, onSelect: c.remove },
    ];
    case 'tag': return [{ id: 'filter', label: t('smenu.filterTag'), onSelect: c.filterByTag }];
    case 'recent': return [open(c), sep, { id: 'clear', label: t('smenu.clearRecent'), danger: true, disabled: !c.recentCount, reason: t('smenu.empty'), onSelect: c.clearRecent }];
    case 'active': return [open(c), sep, { id: 'off', label: t('smenu.deactivateAllN', { n: c.activeCount ?? 0 }), disabled: !c.activeCount, reason: t('smenu.empty'), onSelect: c.deactivateAll }];
    case 'compare': return [{ id: 'resume', label: t('smenu.resume'), onSelect: c.open }, sep, { id: 'close', label: t('smenu.closeSession'), danger: true, onSelect: c.closeSession }];
  }
}

/** Số font từ đó hỏi xác nhận trước khi tắt hàng loạt. */
export const CONFIRM_DEACTIVATE_FROM = 50;
