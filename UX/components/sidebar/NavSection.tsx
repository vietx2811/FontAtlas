import type { ReactNode } from 'react';
import { Icon } from './icons';

type Props = { title: string; open: boolean; onToggle(): void; addLabel?: string; onAdd?(): void; children: ReactNode };

export function NavSection({ title, open, onToggle, addLabel, onAdd, children }: Props): React.JSX.Element {
  return (
    <section className="fa-sec" data-open={open}>
      <div className="fa-sec-row">
        <button type="button" className="fa-sec-head" aria-expanded={open} onClick={onToggle}>
          <span className="fa-tri" aria-hidden /> <span>{title}</span>
        </button>
        {onAdd && <button type="button" className="fa-sec-add" aria-label={addLabel} title={addLabel} onClick={onAdd}><Icon name="plus" size={14} /></button>}
      </div>
      <div className="fa-sec-body"><div inert={!open}>{children}</div></div>
    </section>
  );
}
