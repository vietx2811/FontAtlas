// Chữ riêng theo nền tảng: các chuỗi viết cho macOS (Finder, Thùng rác, "máy Mac"…) đổi từ ngữ khi chạy trên Windows/Linux.
// macOS không đi qua đây (xem `platformText`), nên bản mac không đổi một chữ nào.
import { getOs } from '../platform';
import type { Locale } from './locale';

type Rule = [RegExp, string];
const WIN_EN: Rule[] = [[/Show in Finder/g, 'Show in Explorer'], [/\bFinder\b/g, 'File Explorer'], [/\bthe Trash\b/g, 'the Recycle Bin'], [/\bTrash\b/g, 'Recycle Bin'], [/\bmacOS\b/g, 'Windows']];
const MAC_NOUN_EN = /\b(this|your|the|a) Mac(?!\w)/g;
const RULES: Record<string, Record<Locale, Rule[]>> = {
  windows: {
    en: [...WIN_EN, [MAC_NOUN_EN, '$1 PC']],
    vi: [[/Hiện trong Finder/g, 'Hiện trong Explorer'], [/\bFinder\b/g, 'File Explorer'], [/\bmacOS\b/g, 'Windows'], [/\bmáy Mac\b/g, 'máy tính']],
  },
  linux: {
    en: [[/Show in Finder/g, 'Show in file manager'], [/\bFinder\b/g, 'file manager'], [/\bmacOS\b/g, 'your system'], [MAC_NOUN_EN, '$1 computer']],
    vi: [[/Hiện trong Finder/g, 'Hiện trong trình quản lý file'], [/\bFinder\b/g, 'trình quản lý file'], [/\bmacOS\b/g, 'hệ điều hành'], [/\bmáy Mac\b/g, 'máy tính']],
  },
};
/** Chuỗi đã chỉnh cho nền tảng hiện tại; macOS trả đúng chuỗi gốc. */
export function platformText(s: string, locale: Locale): string {
  const os = getOs();
  if (os === 'mac') return s;
  let out = s;
  for (const [re, to] of RULES[os]?.[locale] ?? []) out = out.replace(re, to);
  return out;
}
