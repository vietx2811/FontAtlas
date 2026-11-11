// Bố cục lưới theo nhóm Family (R3): tiêu đề nhóm + các hàng thẻ, dựng từ `familyGroups(q)` mà không tải face.
export type FamilyGroup = { family: string; start: number; count: number; matchCount: number; styleTotal: number };
/** `head`: tiêu đề của nhóm g. `cards`: hàng thẻ, `first` = chỉ số face (theo thứ tự `queryFaces`) của thẻ đầu hàng, `n` = số thẻ thật trong hàng. */
export type LayoutRow = { kind: 'head'; g: number } | { kind: 'cards'; g: number; first: number; n: number };

/** Chiều cao tiêu đề nhóm (px), cố định để `rowHeight` của lưới ảo hoá là hàm theo chỉ số. */
export const HEADER_H = 32;

/** Mỗi nhóm bắt đầu hàng mới; hàng cuối nhóm không lấp bằng thẻ nhóm sau; nhóm thu gọn chỉ còn tiêu đề. */
export function buildLayout(groups: readonly FamilyGroup[], cols: number, collapsed: ReadonlySet<string>): LayoutRow[] {
  const out: LayoutRow[] = [];
  const c = Math.max(1, cols);
  groups.forEach((g, gi) => {
    out.push({ kind: 'head', g: gi });
    if (collapsed.has(g.family)) return;
    for (let k = 0; k < g.count; k += c) out.push({ kind: 'cards', g: gi, first: g.start + k, n: Math.min(c, g.count - k) });
  });
  return out;
}

/** Nhóm dùng được khi phủ đúng toàn bộ kết quả, liên tục từ 0 (nếu lệch — truy vấn vừa đổi — lưới rơi về dạng phẳng). */
export function groupsCover(groups: readonly FamilyGroup[] | null, total: number | null): groups is readonly FamilyGroup[] {
  if (!groups || total === null || total === 0) return false;
  let at = 0;
  for (const g of groups) { if (g.start !== at || g.count <= 0) return false; at += g.count; }
  return at === total;
}

/** Nhãn số kiểu: "9 kiểu", hoặc "4/9 kiểu" khi bộ lọc đang ẩn bớt. */
export const styleCountParts = (g: FamilyGroup): { shown: number; total: number; filtered: boolean } => ({ shown: g.matchCount, total: g.styleTotal, filtered: g.matchCount < g.styleTotal });

/** Khoảng chỉ số face của các hàng đang thấy (để nạp trang): mỗi hàng thẻ một khoảng, mỗi tiêu đề một điểm (font đại diện). */
export function faceSpansOf(layout: readonly LayoutRow[], groups: readonly FamilyGroup[], start: number, stop: number): Array<[number, number]> {
  const out: Array<[number, number]> = [];
  for (let i = Math.max(0, start); i <= Math.min(stop, layout.length - 1); i++) {
    const r = layout[i];
    if (r.kind === 'head') { const s = groups[r.g].start; out.push([s, s]); } else out.push([r.first, r.first + r.n - 1]);
  }
  return out;
}

/** Khoảng chỉ số face [a, b] (đóng), đã sắp theo a và không chồng nhau — các nhóm đang thu gọn. */
export type Ranges = ReadonlyArray<readonly [number, number]>;

/** Tìm nhị phân: chỉ số i có nằm trong một khoảng không (O(log R)). */
export function inRanges(ranges: Ranges, i: number): boolean {
  let lo = 0; let hi = ranges.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1; const [a, b] = ranges[mid];
    if (i < a) hi = mid - 1; else if (i > b) lo = mid + 1; else return true;
  }
  return false;
}

/** Các đoạn [a, b] của chỉ số ĐANG THẤY trong [0, total): phần bù của các khoảng ẩn. */
export function visibleSegments(total: number, hidden: Ranges): Array<[number, number]> {
  const out: Array<[number, number]> = []; let at = 0;
  for (const [a, b] of hidden) {
    if (a > at) out.push([at, Math.min(a - 1, total - 1)]);
    at = Math.max(at, b + 1);
    if (at >= total) break;
  }
  if (at < total) out.push([at, total - 1]);
  return out.filter(([a, b]) => b >= a);
}

/** Chỉ số hàng chứa face `face` trong bố cục (nhóm thu gọn → hàng tiêu đề của nó); 0 nếu không thấy. Dùng để giữ vị trí cuộn khi bố cục đổi. */
export function rowOfFace(layout: readonly LayoutRow[], groups: readonly FamilyGroup[], face: number): number {
  for (let i = 0; i < layout.length; i++) {
    const r = layout[i];
    if (r.kind === 'cards') { if (face >= r.first && face < r.first + r.n) return i; }
    else { const g = groups[r.g]; if (face >= g.start && face < g.start + g.count && (layout[i + 1]?.kind !== 'cards' || (layout[i + 1] as { g: number }).g !== r.g)) return i; }
  }
  return 0;
}
