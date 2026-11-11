import { useEffect, useState } from 'react';
import type { Api } from '../../api';
import type { FontFaceLoader } from '../../features/library-grid/fontFaceLoader';
import { sharedFontLoader } from '../../features/library-grid/sharedLoader';
import { useLoadedFamily } from '../../features/library-grid/useLoadedFamily';

type Face = { hashHex: string; faceIndex: number; fileSize: number };

/** Bộ nạp dùng chung với lưới (một hàng đợi, một ngân sách). */
export const loaderFor = (api: Api): FontFaceLoader => sharedFontLoader(api).loader;

/** Họ font đã nạp từ fontatlas-font:// cho một face dùng làm NỘI DUNG (specimen…) → ref cứng, ưu tiên cao. null: chưa nạp, hoặc face ≥ 1 của TTC. */
export function useFontFamily(api: Api, face: Face | null): string | null {
  return useLoadedFamily(loaderFor(api), face);
}

export type FontStatus = { family: string | null; status: 'none' | 'loading' | 'ready' | 'failed' };
/**
 * Như useFontFamily nhưng nói rõ đang tải / xong / không nạp được. Font còn trên cloud đi qua cổng tải của backend
 * (≤ 4 đồng thời, mỗi lần 30 s, theo công tắc "Tự tải font trên cloud để xem trước"), nên chờ lâu hoặc bị tắt đều kết thúc bằng `failed`.
 */
export function useFontStatus(api: Api, face: Face | null): FontStatus {
  const loader = loaderFor(api);
  const [st, setSt] = useState<FontStatus>({ family: null, status: face ? 'loading' : 'none' });
  const hash = face?.hashHex; const index = face?.faceIndex; const size = face?.fileSize ?? 0;
  useEffect(() => {
    if (hash === undefined || index === undefined) { setSt({ family: null, status: 'none' }); return; }
    setSt({ family: null, status: 'loading' });
    const row = { hashHex: hash, faceIndex: index, fileSize: size };
    let alive = true;
    loader.acquire(row).then(f => { if (alive) setSt(f ? { family: f, status: 'ready' } : { family: null, status: 'failed' }); }, () => { if (alive) setSt({ family: null, status: 'failed' }); });
    return () => { alive = false; loader.release(row); };
  }, [loader, hash, index, size]);
  return st;
}
