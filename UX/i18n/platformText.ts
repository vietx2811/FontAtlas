// Chữ riêng theo nền tảng: các chuỗi viết cho macOS (Finder, Thùng rác, "máy Mac"…) đổi từ ngữ khi chạy trên Windows/Linux.
// macOS không đi qua đây (xem `platformText`), nên bản mac không đổi một chữ nào.
import { getOs } from '../platform';
import type { Locale } from './locale';

type Rule = [RegExp, string];
// Chữ "Mac" đứng riêng: "this Mac", "This Mac" đầu câu, "on Mac", "trên Mac" (không đụng "macOS", "MacBook").
const MAC_DET = /\b(this|your|the|a) Mac(?!\w)/g;
const MAC_START = /(^|[.!?]\s+)(This|Your|The|A) Mac(?!\w)/g; // đầu câu; không đụng "MAC address"
const MAC_ALONE = /\bMac(?!\w)/g;
const RULES: Record<string, Record<Locale, Rule[]>> = {
  windows: {
    en: [[/Show in Finder/g, 'Show in Explorer'], [/\bFinder\b/g, 'File Explorer'], [/\bthe Trash\b/g, 'the Recycle Bin'], [/\bTrash\b/g, 'Recycle Bin'], [/\bmacOS\b/g, 'Windows'], [MAC_DET, '$1 PC'], [MAC_START, '$1$2 PC'], [MAC_ALONE, 'PC']],
    vi: [[/Hiện trong Finder/g, 'Hiện trong Explorer'], [/\bFinder\b/g, 'File Explorer'], [/\bmacOS\b/g, 'Windows'], [/\bmáy Mac\b/g, 'máy tính'], [MAC_ALONE, 'PC']],
  },
  linux: {
    en: [[/Show in Finder/g, 'Show in file manager'], [/\bFinder\b/g, 'file manager'], [/\bmacOS\b/g, 'your system'], [MAC_DET, '$1 computer'], [MAC_START, '$1$2 computer'], [MAC_ALONE, 'computer']],
    vi: [[/Hiện trong Finder/g, 'Hiện trong trình quản lý file'], [/\bFinder\b/g, 'trình quản lý file'], [/\bmacOS\b/g, 'hệ điều hành'], [/\bmáy Mac\b/g, 'máy tính'], [MAC_ALONE, 'máy tính']],
  },
};
const HINT = /Finder|Trash|Mac|macOS/; // không có dấu hiệu nào thì khỏi chạy luật
const memo = new Map<string, string>();
/** Chuỗi MẪU đã chỉnh cho nền tảng hiện tại (gọi TRƯỚC khi nội suy {biến}); macOS trả đúng chuỗi gốc. */
export function platformText(s: string, locale: Locale): string {
  const os = getOs();
  if (os === 'mac' || !HINT.test(s)) return s;
  const k = `${os}|${locale}|${s}`;
  let out = memo.get(k);
  if (out === undefined) {
    out = s;
    for (const [re, to] of RULES[os]?.[locale] ?? []) out = out.replace(re, to);
    memo.set(k, out);
  }
  return out;
}
