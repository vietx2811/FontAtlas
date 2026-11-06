import { useEffect, useState } from 'react';
import type { FontFaceLoader, LoaderRow } from './fontFaceLoader';

/** `font-family` dùng chính font đã nạp, rồi tới font giao diện cho glyph thiếu (font ký hiệu, không có Latin). */
export const ownFontStack = (family: string): string => `"${family}", var(--font-ui)`;

/**
 * Nạp font của một face: acquire khi hiện, release khi rời.
 * `soft` = chỉ để vẽ TÊN (làn ưu tiên thấp, bỏ qua font quá lớn, có thể bị gỡ khi vượt ngân sách).
 * Trả null khi chưa nạp xong, bị bỏ qua lúc cuộn nhanh, quá lớn, hoặc face ≥ 1 của TTC (FontFace không chọn được face).
 */
export function useLoadedFamily(loader: FontFaceLoader | null, face: LoaderRow | null, soft = false): string | null {
  const [family, setFamily] = useState<string | null>(null);
  const hash = face?.hashHex; const index = face?.faceIndex; const size = face?.fileSize ?? 0;
  useEffect(() => {
    setFamily(null);
    if (!loader || hash === undefined || index === undefined) return;
    const row = { hashHex: hash, faceIndex: index, fileSize: size };
    let alive = true;
    loader.acquire(row, { soft }).then(f => { if (alive) setFamily(f); }, () => { /* giữ font giao diện */ });
    return () => { alive = false; loader.release(row, { soft }); };
  }, [loader, hash, index, size, soft]);
  return family;
}
