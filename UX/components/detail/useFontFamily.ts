import { useEffect, useState } from 'react';
import type { Api } from '../../api';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from '../../features/library-grid/fontFaceLoader';

type Face = { hashHex: string; faceIndex: number; fileSize: number };
const loaders = new WeakMap<Api, FontFaceLoader>();
const loaderFor = (api: Api): FontFaceLoader => {
  let l = loaders.get(api);
  if (!l) { l = new FontFaceLoader(browserLoaderDeps(api, new ScrollVelocity())); loaders.set(api, l); }
  return l;
};

/** Họ font đã nạp từ fontatlas-font:// cho một face (null: chưa nạp, hoặc face ≥ 1 của TTC → không dùng FontFace). */
export function useFontFamily(api: Api, face: Face | null): string | null {
  const [family, setFamily] = useState<string | null>(null);
  const hash = face?.hashHex; const index = face?.faceIndex; const size = face?.fileSize ?? 0;
  useEffect(() => {
    setFamily(null);
    if (hash === undefined || index === undefined) return;
    const loader = loaderFor(api);
    const row = { hashHex: hash, faceIndex: index, fileSize: size };
    let alive = true;
    loader.acquire(row).then(f => { if (alive) setFamily(f); }, () => { /* giữ font giao diện */ });
    return () => { alive = false; loader.release(row); };
  }, [api, hash, index, size]);
  return family;
}
