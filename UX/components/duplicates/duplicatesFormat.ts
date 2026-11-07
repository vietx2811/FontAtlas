import type { DuplicateGroupDto } from '../../api';

const nf = new Intl.NumberFormat('vi-VN', { maximumFractionDigits: 3, useGrouping: false });
export const revisionText = (r: number | null): string => (r === null || !Number.isFinite(r) ? '—' : nf.format(r));
export const dateText = (sec: number | null): string => (sec === null || !Number.isFinite(sec) ? '—' : new Date(sec * 1000).toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit', year: 'numeric' }));

const graphemes = (s: string): string[] => {
  const seg = (Intl as { Segmenter?: new (l?: string, o?: object) => { segment(s: string): Iterable<{ segment: string }> } }).Segmenter;
  return seg ? [...new seg('vi', { granularity: 'grapheme' }).segment(s)].map(x => x.segment) : [...s.normalize('NFC')];
};
/** Cắt giữa đường dẫn: giữ đầu (ổ/thư mục gốc) và đuôi (tên file), không chẻ chữ + dấu. Đầy đủ nằm ở tooltip. */
export function midEllipsis(path: string, max: number): string {
  const g = graphemes(path);
  if (g.length <= max) return path;
  const tail = Math.ceil((max - 1) * 0.65), head = max - 1 - tail;
  return `${g.slice(0, head).join('')}…${g.slice(g.length - tail).join('')}`;
}

/** same_file / formats: tên font; versions: PostScript name (khoá). Loại lạ hoặc nhóm rỗng → khoá. */
export function groupTitle(g: DuplicateGroupDto): string {
  const f = g.faces[0]?.row;
  if (g.kind === 'versions' || !f) return g.key;
  return `${f.family}${f.subfamily ? ` ${f.subfamily}` : ''}`;
}
