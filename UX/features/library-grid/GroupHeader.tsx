import { t } from '../../i18n';
import { Tooltip } from '../../components/ui/Tooltip';
import type { FontFaceLoader } from './fontFaceLoader';
import { styleCountParts, type FamilyGroup } from './groupLayout';
import { ownFontStack, useLoadedFamily } from './useLoadedFamily';
import type { FaceRow } from './types';

type Props = { group: FamilyGroup; rep: FaceRow | undefined; loader: FontFaceLoader | null; collapsed: boolean; onToggle(group: FamilyGroup, collapse: boolean): void; tabStop: boolean; onFocusHead(family: string): void; onNavigate(delta: number): void; style: React.CSSProperties };

/** Tiêu đề nhóm Family (R3): nút thu gọn (`aria-expanded`), tên vẽ bằng chính font đại diện, "N kiểu" hoặc "m/N kiểu" khi đang lọc. ←/→ đóng/mở, ↑/↓ sang tiêu đề kế; cả dải tiêu đề là một tab-stop (roving tabindex). */
export function GroupHeader({ group, rep, loader, collapsed, onToggle, tabStop, onFocusHead, onNavigate, style }: Props): React.JSX.Element {
  const own = useLoadedFamily(loader, rep ? { hashHex: rep.hashHex, faceIndex: rep.faceIndex, fileSize: rep.fileSize } : null, true);
  const c = styleCountParts(group);
  const label = c.filtered ? t('group.styles.of', { m: c.shown, n: c.total }) : t('group.styles', { n: c.total });
  return (
    <div role="row" style={style} className="fa-grouprow">
      <div role="gridcell" className="fa-grouprow-cell">
        <button type="button" className="fa-grouphead" data-testid="group-head" data-family={group.family} aria-expanded={!collapsed}
          tabIndex={tabStop ? 0 : -1} onFocus={() => onFocusHead(group.family)}
          onClick={() => onToggle(group, !collapsed)}
          onKeyDown={e => {
            if (e.key === 'ArrowLeft' && !collapsed) { e.preventDefault(); onToggle(group, true); }
            else if (e.key === 'ArrowRight' && collapsed) { e.preventDefault(); onToggle(group, false); }
            else if (e.key === 'ArrowDown') { e.preventDefault(); onNavigate(1); }
            else if (e.key === 'ArrowUp') { e.preventDefault(); onNavigate(-1); }
          }}>
          <span className="fa-gh-chev" aria-hidden>{collapsed ? '▸' : '▾'}</span>
          <span className={`fa-gh-name${own ? ' fa-own-font' : ''}`} data-testid="group-name" data-own-font={own ?? undefined} style={own ? { fontFamily: ownFontStack(own) } : undefined}>{group.family}</span>
          <Tooltip label={t('group.styles.tip')}><span className="fa-gh-count" data-testid="group-count" data-filtered={c.filtered || undefined}>{label}</span></Tooltip>
        </button>
      </div>
    </div>
  );
}
