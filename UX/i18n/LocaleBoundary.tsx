import { Fragment, type ReactNode } from 'react';
import { useLocale } from './locale';

/** Đổi ngôn ngữ thì dựng lại cây giao diện: mọi chuỗi gọi `t()` lúc render nên cập nhật ngay, không cần khởi động lại. Store (chọn, bộ lọc…) giữ nguyên. */
export function LocaleBoundary({ children }: { children: ReactNode }): React.JSX.Element {
  return <Fragment key={useLocale()}>{children}</Fragment>;
}
