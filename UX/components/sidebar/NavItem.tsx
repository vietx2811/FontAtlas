import type { ReactNode } from 'react';
import { Icon, type IconName } from './icons';

type Props = {
  icon?: IconName; swatch?: string | null; label: string; count?: number | null; current?: boolean;
  unavailable?: boolean; title?: string; kbd?: string; onClick(): void; more?: ReactNode; /** Dòng phụ nhỏ dưới mục (không bấm được). */ sub?: ReactNode;
};

export function NavItem({ icon, swatch, label, count, current, unavailable, title, kbd, onClick, more, sub }: Props): React.JSX.Element {
  return (
    <div className="fa-nav-wrap">
      <button type="button" className={`fa-nav${unavailable ? ' fa-ejected' : ''}`} aria-current={current ? 'true' : undefined}
        data-unavailable={unavailable ? 'true' : undefined} title={title ?? label} aria-label={count != null ? `${label} ${count}` : label} onClick={onClick}>
        {swatch !== undefined && <span className="fa-swatch" style={{ background: swatch ?? 'var(--muted)' }} aria-hidden />}
        {icon && <Icon name={icon} />}
        <span className="fa-lbl">{label}</span>
        {kbd && <span className="fa-kbd">{kbd}</span>}
        {count != null && <span className="fa-n">{count}</span>}
      </button>
      {more}
      {sub}
    </div>
  );
}
