import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, AxisHistogramDto, FaceQuery } from '../../api';

export const HISTOGRAM_DEBOUNCE_MS = 160;

/**
 * Histogram của 18 trục cho tập đang lọc. Đổi bộ lọc → đợi HISTOGRAM_DEBOUNCE_MS rồi mới hỏi; yêu cầu cũ (kể cả đang bay)
 * bị bỏ kết quả khi có yêu cầu mới hơn, nên kéo slider liên tục không làm histogram nhấp nháy theo dữ liệu cũ.
 * Giữ dữ liệu cũ trong lúc tải (loading=true) để UI không khoá và không nhảy bố cục.
 */
export function useAxisHistograms(api: Api, query: FaceQuery, enabled: boolean, refreshKey: unknown = 0): { data: Map<string, AxisHistogramDto>; loading: boolean; error: string | null; retry(): void } {
  const key = JSON.stringify(query);
  const q = useMemo(() => JSON.parse(key) as FaceQuery, [key]);
  const [data, setData] = useState<Map<string, AxisHistogramDto>>(new Map());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tries, setTries] = useState(0);
  const seq = useRef(0);
  useEffect(() => {
    if (!enabled) return;
    const mine = ++seq.current;
    setLoading(true);
    const timer = setTimeout(() => {
      api.axisHistogram(q).then(
        list => { if (seq.current !== mine) return; setData(new Map(list.map(h => [h.axis, h]))); setError(null); setLoading(false); },
        e => { if (seq.current !== mine) return; setError(e instanceof Error ? e.message : String(e)); setLoading(false); },
      );
    }, HISTOGRAM_DEBOUNCE_MS);
    return () => { clearTimeout(timer); if (seq.current === mine) seq.current++; };
  }, [api, q, enabled, refreshKey, tries]);
  return { data, loading, error, retry: () => setTries(n => n + 1) };
}
