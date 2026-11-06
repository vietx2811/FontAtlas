import type { Api } from '../../api';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from '../../features/library-grid/fontFaceLoader';
import { useLoadedFamily } from '../../features/library-grid/useLoadedFamily';

type Face = { hashHex: string; faceIndex: number; fileSize: number };
const loaders = new WeakMap<Api, FontFaceLoader>();
export const loaderFor = (api: Api): FontFaceLoader => {
  let l = loaders.get(api);
  if (!l) { l = new FontFaceLoader(browserLoaderDeps(api, new ScrollVelocity())); loaders.set(api, l); }
  return l;
};

/** Họ font đã nạp từ fontatlas-font:// cho một face (null: chưa nạp, hoặc face ≥ 1 của TTC → không dùng FontFace). */
export function useFontFamily(api: Api, face: Face | null): string | null {
  return useLoadedFamily(loaderFor(api), face);
}
