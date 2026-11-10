import type { AxisHistogramDto } from '../../api';

/** Histogram nằm sau track: lớp nhạt = `spanBins` (gồm Variable phủ nấc), lớp đậm = `bins`. Trục chưa có số đo (`empty`) không vẽ gì. */
export function Histogram({ hist }: { hist: Pick<AxisHistogramDto, 'bins' | 'spanBins' | 'empty'> }): React.JSX.Element | null {
  if (hist.empty || !hist.bins.length) return null;
  const n = hist.bins.length;
  const top = Math.max(1, ...hist.bins, ...hist.spanBins);
  return (
    <svg className="fa-hist" data-testid="histogram" viewBox={`0 0 ${n} 1`} preserveAspectRatio="none" width="100%" height="100%" aria-hidden>
      {hist.spanBins.map((v, i) => v > hist.bins[i] ? <rect key={`s${i}`} className="fa-hist-span" x={i + 0.08} width={0.84} y={1 - v / top} height={v / top} /> : null)}
      {hist.bins.map((v, i) => <rect key={`b${i}`} className="fa-hist-bin" x={i + 0.08} width={0.84} y={1 - v / top} height={v / top} />)}
    </svg>
  );
}
