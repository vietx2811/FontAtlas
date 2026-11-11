import type { ReactNode } from 'react';
import { Icon, type IconName } from './icons';
import { useUiStore } from '../../store/ui';
import { Tooltip } from '../ui/Tooltip';

type Props = {
  icon?: IconName; swatch?: string | null; label: string; count?: number | null; current?: boolean;
  unavailable?: boolean; title?: string; kbd?: string; onClick(): void;
  /** Chuột phải, phím Menu hoặc ⇧F10; `rect` có khi mở bằng bàn phím. Không có → không chặn menu hệ thống. */
  onContextMenu?(at: { x: number; y: number; keyboard: boolean; el: HTMLElement }): void; onRename?(): void; /** Khoá để tìm lại nút (trả focus sau khi đổi tên). */ navId?: string; /** Mục đang được menu nhắm tới. */ context?: boolean; more?: ReactNode; /** Dòng phụ nhỏ dưới mục (không bấm được). */ sub?: ReactNode;
};

export function NavItem({ icon, swatch, label, count, current, unavailable, title, kbd, onClick, more, sub, onContextMenu, onRename, context, navId }: Props): React.JSX.Element {
  const collapsed = useUiStore(s => s.sidebarCollapsed);
  return (
    <div className="fa-nav-wrap">
      {/* Thanh bên thu gọn chỉ còn icon → luôn có tooltip; khi mở rộng chỉ hiện nếu có chú thích riêng (đường dẫn, trạng thái). */}
      <Tooltip label={title ?? label} disabled={!collapsed && title === undefined}>
      <button type="button" className={`fa-nav${unavailable ? ' fa-ejected' : ''}`} aria-current={current ? 'true' : undefined}
        data-unavailable={unavailable ? 'true' : undefined} aria-label={count != null ? `${label} ${count}` : label} onClick={onClick}
        data-context={context ? 'true' : undefined} data-nav-id={navId}
        onContextMenu={onContextMenu ? e => { e.preventDefault(); onContextMenu({ x: e.clientX, y: e.clientY, keyboard: false, el: e.currentTarget }); } : undefined}
        onKeyDown={e => {
          if (onContextMenu && (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey))) { e.preventDefault(); const r = e.currentTarget.getBoundingClientRect(); onContextMenu({ x: r.left + 24, y: r.bottom - 4, keyboard: true, el: e.currentTarget }); }
          else if (onRename && e.key === 'F2') { e.preventDefault(); onRename(); }
        }}>
        {swatch !== undefined && <span className="fa-swatch" style={{ background: swatch ?? 'var(--muted)' }} aria-hidden />}
        {icon && <Icon name={icon} />}
        <span className="fa-lbl">{label}</span>
        {kbd && <span className="fa-kbd">{kbd}</span>}
        {count != null && <span className="fa-n">{count}</span>}
      </button>
      </Tooltip>
      {more}
      {sub}
    </div>
  );
}
