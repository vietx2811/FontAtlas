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
