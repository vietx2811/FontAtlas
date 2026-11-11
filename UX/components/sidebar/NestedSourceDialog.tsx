import { useEffect, useRef } from 'react';
import type { SourceDto } from '../../api';
import { t } from '../../i18n';

export type Nesting = 'inside' | 'contains';
type Props = {
  newPath: string; existing: SourceDto; nesting: Nesting;
  onCancel(): void; onCreateCollection(name: string): void; onReplace(): void;
};
const baseName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

/** D4 (UX 03 §2.3): thư mục mới nằm trong hoặc chứa một nguồn đã có. */
export function NestedSourceDialog({ newPath, existing, nesting, onCancel, onCreateCollection, onReplace }: Props): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { box.current?.querySelector<HTMLButtonElement>('button')?.focus(); }, []);
  const name = baseName(newPath);
  const title = nesting === 'inside' ? t('nsd.titleInside') : t('nsd.titleContains', { name: existing.name });
  return (
    <div className="fa-modal-scrim" onMouseDown={e => { if (e.target === e.currentTarget) onCancel(); }}>
      <div ref={box} role="dialog" aria-modal="true" aria-labelledby="d4-title" className="fa-modal" onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onCancel(); } }}>
        <h2 id="d4-title">{title}</h2>
        <dl className="fa-d4-paths">
          <dt>{t('nsd.new')}</dt><dd>{newPath}</dd>
          <dt>{t('nsd.existing')}</dt><dd>{existing.path}<span className="fa-d4-count">{t('nsd.count', { n: existing.faceCount })}</span></dd>
        </dl>
        {nesting === 'inside' ? (
          <p>{t('nsd.bodyInside', { name, source: existing.name })}</p>
        ) : (
          <p>{t('nsd.bodyReplace')}</p>
        )}
        <div className="fa-modal-actions">
          <button type="button" onClick={onCancel}>{t('nsd.cancel')}</button>
          {nesting === 'inside'
            ? <button type="button" className="primary" onClick={() => onCreateCollection(name)}>{t('nsd.createCollection', { name })}</button>
            : <button type="button" className="primary" onClick={onReplace}>{t('nsd.replace')}</button>}
        </div>
      </div>
    </div>
  );
}
