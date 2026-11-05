import { useState } from 'react';
import type { FaceDetailDto } from '../../api';

type Props = { detail: FaceDetailDto | null; expanded: boolean; onToggle(): void; onReveal(): void };
const clean = (v: string | null | undefined): string | null => (v && v.trim() ? v.trim() : null);

export function FontInfoCard({ detail, expanded, onToggle, onReveal }: Props): React.JSX.Element | null {
  const [menu, setMenu] = useState(false);
  if (!detail) return null;
  const m = detail.metadata;
  const base: Array<[string, string | null, string?]> = [['Thiết kế', clean(m.designer)], ['Giấy phép', clean(m.license)]];
  const more: Array<[string, string | null, string?]> = [
    ['Hãng', clean(m.manufacturer)], ['Phiên bản', clean(m.version)], ['Bản quyền', clean(m.copyright), 'soft'], ['Tệp', clean(detail.file.path), 'mono'],
  ];
  const hasMore = more.some(([, v]) => v);
  const rows = [...base, ...(expanded ? more : [])].filter(([, v]) => v);
  return (
    <dl className="fa-meta-card">
      {rows.map(([k, v, cls]) => (
        <div key={k}>
          <dt>{k}</dt>
          <dd className={cls} title={cls === 'mono' ? v! : undefined} onContextMenu={cls === 'mono' ? e => { e.preventDefault(); setMenu(true); } : undefined}>{v}</dd>
          {k === 'Tệp' && menu && (
            <div role="menu" className="fa-ctx" onMouseLeave={() => setMenu(false)}>
              <button role="menuitem" type="button" onClick={() => { setMenu(false); onReveal(); }}>Hiện trong Finder</button>
            </div>
          )}
        </div>
      ))}
      {hasMore && (
        <button type="button" className="meta-toggle" aria-expanded={expanded} onClick={onToggle}>
          {expanded ? 'Thu gọn' : 'Hãng · Phiên bản · Bản quyền · Tệp'}<span className="chev" aria-hidden />
        </button>
      )}
    </dl>
  );
}
