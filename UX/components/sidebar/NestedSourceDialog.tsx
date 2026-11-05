import { useEffect, useRef } from 'react';
import type { SourceDto } from '../../api';

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
  const title = nesting === 'inside' ? 'Thư mục này nằm trong một nguồn đã có' : `Thư mục này chứa nguồn “${existing.name}” đã có`;
  return (
    <div className="fa-modal-scrim" onMouseDown={e => { if (e.target === e.currentTarget) onCancel(); }}>
      <div ref={box} role="dialog" aria-modal="true" aria-labelledby="d4-title" className="fa-modal" onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onCancel(); } }}>
        <h2 id="d4-title">{title}</h2>
        <dl className="fa-d4-paths">
          <dt>Mới</dt><dd>{newPath}</dd>
          <dt>Đã có</dt><dd>{existing.path}<span className="fa-d4-count"> ({existing.faceCount} font)</span></dd>
        </dl>
        {nesting === 'inside' ? (
          <p>Font trong “{name}” đã có trong thư viện qua nguồn “{existing.name}”. Muốn nhóm riêng font này, hãy tạo một collection.</p>
        ) : (
          <p>Thay nguồn cũ bằng thư mục này: yêu thích, collection và thẻ được giữ vì khoá theo hash; chỉ quét lại phần mới.</p>
        )}
        <div className="fa-modal-actions">
          <button type="button" onClick={onCancel}>Huỷ</button>
          {nesting === 'inside'
            ? <button type="button" className="primary" onClick={() => onCreateCollection(name)}>Tạo collection “{name}”</button>
            : <button type="button" className="primary" onClick={onReplace}>Thay nguồn cũ bằng thư mục này</button>}
        </div>
      </div>
    </div>
  );
}
