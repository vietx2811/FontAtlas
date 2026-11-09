import { within } from '@testing-library/react';

/** Các NoteMark (dấu "!") có nhãn chứa `text` (chuỗi con hoặc regex), trong `root` (mặc định cả trang). */
export function notes(text: string | RegExp, root: HTMLElement = document.body): HTMLElement[] {
  return within(root).queryAllByRole('img').filter(e => e.classList.contains('fa-note') && (typeof text === 'string' ? (e.getAttribute('aria-label') ?? '').includes(text) : text.test(e.getAttribute('aria-label') ?? '')));
}
