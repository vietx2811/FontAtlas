import { createPortal } from 'react-dom';
import { formatNumber, t, tn } from '../../i18n';
import { faceName } from '../faces/faceMenuModel';
import { useFaceDragStore } from '../../store/faceDrag';
import './dragLayer.css';

/** "Con trỏ báo số font" khi kéo font vào thanh bên (G1): số font đang kéo + đích đang rê qua. */
export function DragLayer(): React.JSX.Element | null {
  const drag = useFaceDragStore(s => s.drag);
  if (!drag) return null;
  const n = drag.rows.length;
  const over = drag.over;
  return createPortal(
    <div className="fa-draglayer" data-testid="drag-layer" role="status" style={{ transform: `translate(${drag.x + 14}px, ${drag.y + 14}px)` }} data-over={over ? (over.valid ? 'ok' : 'no') : undefined}>
      <span className="fa-draglayer-n" data-testid="drag-count">{formatNumber(n)}</span>
      <span className="fa-draglayer-t">
        {n === 1 ? faceName(drag.rows[0]) : tn('faces.nFonts', { n: formatNumber(n) })}
        {over && <small>{over.valid ? (over.kind === 'active' ? t('dnd.dropActive') : over.kind === 'compare' ? t('dnd.dropCompare') : over.kind === 'role' ? (drag.fromRole ? t('dnd.dropSwap', { role: over.name }) : t('dnd.dropRole', { role: over.name })) : over.kind === 'tag' ? t('dnd.dropTag', { name: over.name }) : t(drag.move ? 'dnd.moveTo' : 'dnd.dropTo', { name: over.name })) : t('dnd.here', { name: over.name })}</small>}
      </span>
    </div>,
    document.body,
  );
}
