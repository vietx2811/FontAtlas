import { Tooltip } from './Tooltip';
import './noteMark.css';

/**
 * Dấu chấm than đỏ nhỏ ở góc ô: cảnh báo / ghi chú phụ không hiện thành chip hay câu chữ.
 * Hover hoặc focus bàn phím (tabIndex) mở tooltip. Đặt trong phần tử cha `position: relative` (class `fa-note-host`).
 * Ảnh PNG xuất ra không có hover nên giữ chữ rõ ràng (không dùng component này).
 */
export function NoteMark({ text, kind = 'warn', inline = false, testId, className }: { text: string; kind?: 'warn' | 'info'; inline?: boolean; testId?: string; className?: string }): React.JSX.Element | null {
  if (!text) return null;
  return (
    <Tooltip label={text}>
      <span className={`fa-note${inline ? ' inline' : ''}${className ? ` ${className}` : ''}`} data-kind={kind} data-testid={testId} role="img" tabIndex={0} aria-label={text}>!</span>
    </Tooltip>
  );
}
