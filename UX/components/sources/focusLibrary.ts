/**
 * Sau khi hộp thoại (Gỡ nguồn…) đóng, Modal trả focus về phần tử đã có trước đó — ở thanh bên. ⌘A / Esc của thư viện chỉ chạy khi focus ở thư viện
 * hoặc ngoài mọi vùng, nên ⌘A không chọn được gì. Chuyển focus về lưới: ô đầu tiên (hoặc ô đang chọn); chưa có ô nào (đang nạp lại) thì thử lại vài khung hình,
 * rồi bỏ focus khỏi thanh bên để phím tắt thư viện vẫn chạy.
 */
export function focusLibraryGrid(tries = 8): void {
  const run = (left: number) => {
    const grid = document.querySelector('[role="grid"]');
    const cell = grid?.querySelector<HTMLElement>('[role="gridcell"][aria-selected="true"]') ?? grid?.querySelector<HTMLElement>('[role="gridcell"]');
    if (cell) { cell.focus(); return; }
    if (left > 0) { requestAnimationFrame(() => run(left - 1)); return; }
    if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body) document.activeElement.blur();
  };
  // Sau khi cleanup của Modal đã trả focus về thanh bên.
  setTimeout(() => run(tries), 0);
}
