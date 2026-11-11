import { Children, useEffect, useRef, useState } from 'react';
import type { ReactElement, ReactNode } from 'react';

export const MASONRY_MIN_COL = 208;
export const MASONRY_GAP = 16;

/**
 * Masonry tất định: chia thẻ vào N cột flex (mỗi cột bắt đầu ngay đỉnh), thẻ kế tiếp vào cột đang thấp nhất theo ước lượng chiều cao
 * `data-weight` (px). Thay `columns: 208px` của CSS vì WebKit cân cột làm cột cuối lệch đỉnh. N = số cột 208px vừa bề rộng.
 */
export function Masonry({ children }: { children: ReactNode }): React.JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  const [cols, setCols] = useState(4);
  useEffect(() => {
    const el = root.current; if (!el) return;
    const measure = () => {
      const cs = getComputedStyle(el); const inner = el.clientWidth - parseFloat(cs.paddingLeft || '0') - parseFloat(cs.paddingRight || '0');
      setCols(Math.max(1, Math.floor((inner + MASONRY_GAP) / (MASONRY_MIN_COL + MASONRY_GAP))));
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure); ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const lanes: ReactElement[][] = Array.from({ length: cols }, () => []);
  const heights = new Array<number>(cols).fill(0);
  for (const child of Children.toArray(children) as ReactElement<{ 'data-weight'?: number }>[]) {
    const k = heights.indexOf(Math.min(...heights));
    lanes[k].push(child); heights[k] += (child.props['data-weight'] ?? 200) + MASONRY_GAP;
  }
  return (
    <div ref={root} className="fa-adv-cols" data-testid="advanced-cols" data-cols={cols}>
      {lanes.map((lane, i) => <div key={i} className="fa-adv-lane">{lane}</div>)}
    </div>
  );
}
