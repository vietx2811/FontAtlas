import type { FaceDetailDto, SimilarSource } from '../../../api';
import { faceKey, useUiStore } from '../../../store/ui';
import { AXIS_META } from './AnatomyTab';

export const SOURCE_BADGE: Record<SimilarSource, string> = { axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' };
const names = (keys: string[]): string => keys.map(k => (AXIS_META[k]?.label ?? k).toLowerCase()).join(', ');

/** Ba font gần nhất. Không điểm, không phần trăm: chỉ "Giống ở / Khác ở" và nguồn phân tích. */
export function SimilarTab({ detail }: { detail: FaceDetailDto }): React.JSX.Element {
  const select = useUiStore(s => s.select);
  if (detail.similarTop.length === 0) return <p className="fa-soon">Font này có quá ít số đo để tìm font tương tự.</p>;
  return (
    <ul className="fa-sim">
      {detail.similarTop.map(h => (
        <li key={faceKey(h.face)} data-testid="similar-hit" className="fa-sim-hit">
          <button type="button" className="fa-sim-name" onClick={() => select(faceKey(h.face))}>
            {h.row.family}{h.row.subfamily ? ` ${h.row.subfamily}` : ''}
          </button>
          <span className="fa-badge" data-testid="source-badge">{SOURCE_BADGE[h.source]}</span>
          <p className="fa-sim-why">Giống ở: {names(h.same)} · Khác ở: {names(h.differ)}</p>
        </li>
      ))}
    </ul>
  );
}
