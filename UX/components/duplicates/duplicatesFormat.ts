import type { DuplicateGroupDto } from '../../api';
import { localeTag } from '../../i18n';

const nf = { format: (r: number): string => new Intl.NumberFormat(localeTag(), { maximumFractionDigits: 3, useGrouping: false }).format(r) };
export const revisionText = (r: number | null): string => (r === null || !Number.isFinite(r) ? '—' : nf.format(r));
export const dateText = (sec: number | null): string => (sec === null || !Number.isFinite(sec) ? '—' : new Date(sec * 1000).toLocaleDateString(localeTag(), { day: '2-digit', month: '2-digit', year: 'numeric' }));

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

/** Tách đường dẫn thành đầu (co lại bằng "…") và đuôi gồm `depth` đoạn cuối (mặc định "thư mục cha/tên tệp") luôn hiện. */
export function splitPath(path: string, depth = 2): { head: string; tail: string } {
  let cut = path.length;
  for (let n = 0; n < depth; n++) { const i = path.lastIndexOf('/', cut - 1); if (i < 0) return { head: '', tail: path }; cut = i; }
  return cut <= 0 ? { head: '', tail: path } : { head: path.slice(0, cut + 1), tail: path.slice(cut + 1) };
}

/** Số đoạn cuối tối thiểu (≥ 2) để mọi đường dẫn trong nhóm có đuôi khác nhau: các bản trùng phải nhìn là phân biệt được. */
export function tailDepth(paths: Array<string | null>): number {
  const list = paths.filter((p): p is string => !!p);
  const max = Math.max(2, ...list.map(p => p.split('/').length - 1));
  for (let d = 2; d <= max; d++) if (new Set(list.map(p => splitPath(p, d).tail)).size === list.length) return d;
  return max;
}
