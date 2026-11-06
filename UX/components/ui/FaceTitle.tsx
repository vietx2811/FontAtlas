import type { CSSProperties, ElementType, ReactNode } from 'react';
import type { Api } from '../../api';
import type { AxisCoordDto } from '../../bindings';
import type { FontFaceLoader } from '../../features/library-grid/fontFaceLoader';
import { ownFontStack, useLoadedFamily } from '../../features/library-grid/useLoadedFamily';
import { loaderFor } from '../detail/useFontFamily';
import './faceTitle.css';

type Face = { hashHex: string; faceIndex: number; fileSize?: number };
type Props = {
  face: Face | null; as?: ElementType; className?: string; style?: CSSProperties; children: ReactNode;
  /** Toạ độ trục của instance variable font (chỉnh độ đậm… trên cùng một file). */
  variation?: readonly AxisCoordDto[] | null; 'data-testid'?: string;
} & ({ api: Api; loader?: undefined } | { loader: FontFaceLoader | null; api?: undefined });

/**
 * Tên một font vẽ bằng chính font đó: acquire khi hiện, release khi rời. Chưa nạp xong (hoặc cuộn nhanh, hoặc face ≥ 1 của TTC)
 * thì giữ font giao diện; glyph thiếu rơi về font giao diện nhờ ngăn xếp font.
 */
export function FaceTitle({ api, loader, face, as: Tag = 'span', className, style, variation, children, ...rest }: Props): React.JSX.Element {
  const family = useLoadedFamily(loader !== undefined ? loader : api ? loaderFor(api) : null, face && { ...face, fileSize: face.fileSize ?? 0 });
  // Face nạp qua FontFace chỉ khai báo kiểu gốc: đặt weight/style về gốc và tắt in đậm/nghiêng giả.
  const own: CSSProperties | undefined = family
    ? { fontFamily: ownFontStack(family), fontWeight: 400, fontStyle: 'normal', fontSynthesis: 'none', fontVariationSettings: variation?.length ? variation.map(c => `"${c.tag}" ${c.value}`).join(', ') : undefined }
    : undefined;
  return (
    <Tag {...rest} className={`${family ? 'fa-own-font ' : ''}${className ?? ''}`.trim() || undefined} data-own-font={family ?? undefined} style={{ ...style, ...own }}>{children}</Tag>
  );
}
