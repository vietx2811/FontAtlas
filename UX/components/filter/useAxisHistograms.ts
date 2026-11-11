import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, AxisHistogramDto, AxisStatsDto, FaceQuery } from '../../api';

export const HISTOGRAM_DEBOUNCE_MS = 160;

/**
 * Histogram của 18 trục cho tập đang lọc. Đổi bộ lọc → đợi HISTOGRAM_DEBOUNCE_MS rồi mới hỏi; yêu cầu cũ (kể cả đang bay)
 * bị bỏ kết quả khi có yêu cầu mới hơn, nên kéo slider liên tục không làm histogram nhấp nháy theo dữ liệu cũ.
 * Giữ dữ liệu cũ trong lúc tải (loading=true) để UI không khoá và không nhảy bố cục.
 */
export function useAxisHistograms(api: Api, query: FaceQuery, enabled: boolean, refreshKey: unknown = 0): { data: Map<string, AxisHistogramDto>; extent: Map<string, AxisStatsDto>; loading: boolean; error: string | null; retry(): void } {
  const key = JSON.stringify(query);
  const q = useMemo(() => JSON.parse(key) as FaceQuery, [key]);
  const [data, setData] = useState<Map<string, AxisHistogramDto>>(new Map());
  const [extent, setExtent] = useState<Map<string, AxisStatsDto>>(new Map());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tries, setTries] = useState(0);
  const seq = useRef(0);
  useEffect(() => {
    if (!enabled) { seq.current++; setLoading(false); return; }
    const mine = ++seq.current;
    setLoading(true);
    const timer = setTimeout(() => {
      // Cực trị thật (không cắt p1–p99) tính trên tập chưa lọc trục: dùng làm cận mở khi kéo thanh tới mép.
      const { rawAxisRanges: _r, weightClass: _w, widthClass: _d, ...noAxes } = q;
      Promise.all([api.axisHistogram(q), api.axisStats(noAxes)]).then(
        ([list, stats]) => { if (seq.current !== mine) return; setData(new Map(list.map(h => [h.axis, h]))); setExtent(new Map(stats.map(x => [x.axis, x]))); setError(null); setLoading(false); },
        e => { if (seq.current !== mine) return; setError(e instanceof Error ? e.message : String(e)); setLoading(false); },
      );
    }, HISTOGRAM_DEBOUNCE_MS);
    return () => { clearTimeout(timer); if (seq.current === mine) seq.current++; };
  }, [api, q, enabled, refreshKey, tries]);
  return { data, extent, loading, error, retry: () => setTries(n => n + 1) };
}
