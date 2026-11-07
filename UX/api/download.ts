/** Tải một Blob xuống bằng `<a download>` (trình duyệt/dev). Trong WebView Tauri không đáng tin: dùng lệnh lưu file của backend. */
export function downloadBlob(blob: Blob, name: string): string {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a'); a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
  return name;
}
