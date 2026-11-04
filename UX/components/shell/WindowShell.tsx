import type { ReactNode } from 'react';
import './windowShell.css';

/** Bề rộng sidebar (px). Plan 6b G2: sidebar luôn nằm trong layout, không đè nội dung. */
export const SIDEBAR_WIDTH = 248;
/** Chỗ chừa cho nút đèn giao thông của macOS khi titleBarStyle = Overlay. */
export const TRAFFIC_LIGHT_INSET = 78;
/** Chiều cao hàng kéo cửa sổ ở đầu sidebar. */
export const TITLEBAR_HEIGHT = 38;

type Props = {
  sidebar: ReactNode;
  /** Nút đặt ở bên phải hàng kéo (ví dụ nút thu gọn sidebar, T4). */
  titlebarAccessory?: ReactNode;
  children: ReactNode;
};

export function WindowShell({ sidebar, titlebarAccessory, children }: Props): React.JSX.Element {
  return (
    <div data-testid="window-shell" className="fa-shell" style={{ ['--sidebar-w' as string]: `${SIDEBAR_WIDTH}px` }}>
      <aside className="fa-shell-sidebar">
        <div className="fa-shell-titlebar" data-tauri-drag-region>
          {titlebarAccessory}
        </div>
        <div className="fa-shell-sidebar-body">{sidebar}</div>
      </aside>
      <div className="fa-shell-sheet">{children}</div>
    </div>
  );
}
