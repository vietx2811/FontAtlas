import type { ReactNode } from 'react';
import { useLocale } from './locale';

/**
 * Đổi ngôn ngữ thì VẼ LẠI (không dựng lại) cây giao diện: component con được tạo trong hàm `children` nên mọi phần tử mới → React render lại
 * chúng nhưng giữ state/DOM/scroll. Component bọc `memo` hoặc `useMemo` tạo chuỗi phải tự gọi `useLocale()`. Không đặt `key` theo locale.
 */
export function LocaleBoundary({ children }: { children: () => ReactNode }): React.JSX.Element {
  useLocale();
  return <>{children()}</>;
}
