import type { ReactNode } from 'react';
import { Icon } from './icons';
import { Tooltip } from '../ui/Tooltip';

type Props = { title: string; open: boolean; onToggle(): void; addLabel?: string; onAdd?(): void; children: ReactNode };

export function NavSection({ title, open, onToggle, addLabel, onAdd, children }: Props): React.JSX.Element {
  return (
    <section className="fa-sec" data-open={open}>
      <div className="fa-sec-row">
        <button type="button" className="fa-sec-head" aria-expanded={open} onClick={onToggle}>
          <span className="fa-tri" aria-hidden /> <span>{title}</span>
        </button>
        {onAdd && <Tooltip label={addLabel}><button type="button" className="fa-sec-add" aria-label={addLabel} onClick={onAdd}><Icon name="plus" size={14} /></button></Tooltip>}
      </div>
      <div className="fa-sec-body"><div inert={!open}>{children}</div></div>
    </section>
  );
}
