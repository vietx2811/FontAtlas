import type { LibraryApi } from './api';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from './fontFaceLoader';

export type SharedFontLoader = { loader: FontFaceLoader; velocity: ScrollVelocity };
const shared = new WeakMap<object, SharedFontLoader>();

/**
 * MỘT bộ nạp font cho cả ứng dụng (mỗi api): lưới, tên ở panel, danh sách kiểu, Tương tự, Ghép cặp dùng chung ngân sách,
 * dedupe và hàng đợi. Lưới đưa tốc độ cuộn vào `velocity`. Sống suốt vòng đời app nên không dispose.
 */
export function sharedFontLoader(api: LibraryApi): SharedFontLoader {
  let s = shared.get(api);
  if (!s) {
    const velocity = new ScrollVelocity();
    s = { loader: new FontFaceLoader(browserLoaderDeps(api, velocity)), velocity };
    shared.set(api, s);
  }
  return s;
}
