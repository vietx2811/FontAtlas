import { useState } from 'react';
import type { FaceDetailDto } from '../../api';
import { Tooltip } from '../ui/Tooltip';
import { t } from '../../i18n';

type Props = { detail: FaceDetailDto | null; expanded: boolean; onToggle(): void; onReveal(): void };
const clean = (v: string | null | undefined): string | null => (v && v.trim() ? v.trim() : null);

export function FontInfoCard({ detail, expanded, onToggle, onReveal }: Props): React.JSX.Element | null {
  const [menu, setMenu] = useState(false);
  if (!detail) return null;
  const m = detail.metadata;
  const base: Array<[string, string | null, string?]> = [[t('info.designer'), clean(m.designer)], [t('info.license'), clean(m.license)]];
  const more: Array<[string, string | null, string?]> = [
    [t('info.manufacturer'), clean(m.manufacturer)], [t('info.version'), clean(m.version)], [t('info.copyright'), clean(m.copyright), 'soft'], [t('info.file'), clean(detail.file.path), 'mono'],
  ];
  const hasMore = more.some(([, v]) => v);
  const rows = [...base, ...(expanded ? more : [])].filter(([, v]) => v);
  return (
    <dl className="fa-meta-card">
      {rows.map(([k, v, cls]) => (
        <div key={k}>
          <dt>{k}</dt>
          <Tooltip label={v!} onlyIfTruncated><dd className={cls} onContextMenu={cls === 'mono' ? e => { e.preventDefault(); setMenu(true); } : undefined}>{v}</dd></Tooltip>
          {k === t('info.file') && menu && (
            <div role="menu" className="fa-ctx" onMouseLeave={() => setMenu(false)}>
              <button role="menuitem" type="button" onClick={() => { setMenu(false); onReveal(); }}>{t('info.reveal')}</button>
            </div>
          )}
        </div>
      ))}
      {hasMore && (
        <button type="button" className="meta-toggle" aria-expanded={expanded} onClick={onToggle}>
          {expanded ? t('info.collapse') : t('info.expandHint')}<span className="chev" aria-hidden />
        </button>
      )}
    </dl>
  );
}
