import { describe, expect, it } from 'vitest';
import type { FaceMatchDto, FigmaImportDto, FontMatchDto } from '../../../bindings';
import { buildRows, choose, counts, selectedFaces, toggle } from '../figmaImport';
import { parseImportText } from '../../../api/mockFigma';

const f = (h: string, missing = false): FaceMatchDto => ({ face: { hashHex: h, faceIndex: 0 }, family: 'F', style: 'Regular', missing });
const imp: FigmaImportDto = { file: 'Menu v3', items: [
  { family: 'A', style: 'Regular', figmaMissing: false }, { family: 'B', style: 'Bold', figmaMissing: true },
  { family: 'C', style: 'Regular', figmaMissing: false }, { family: 'D', style: 'Light', figmaMissing: true },
] };
const matches: FontMatchDto[] = [
  { index: 0, status: 'exact', faces: [f('aa')] },
  { index: 1, status: 'ambiguous', faces: [f('b1'), f('b2')] },
  { index: 2, status: 'family_only', faces: [f('c1')] },
  { index: 3, status: 'none', faces: [] },
];

describe('nhập từ Figma: dòng xem trước', () => {
  it('chỉ exact được chọn sẵn; ambiguous chờ người dùng; family_only/none không bao giờ vào Case', () => {
    const rows = buildRows(imp, matches);
    expect(rows.map(r => [r.include, r.pick?.hashHex ?? null])).toEqual([[true, 'aa'], [false, null], [false, null], [false, null]]);
    expect(selectedFaces(rows).map(k => k.hashHex)).toEqual(['aa']);
    expect(counts(rows)).toEqual({ total: 4, exact: 1, ambiguous: 1, familyOnly: 1, none: 1, figmaMissing: 2 });
  });
  it('chọn một ứng viên cho ambiguous thì mới vào Case; chọn lại hay bỏ chọn được', () => {
    let rows = buildRows(imp, matches);
    rows = choose(rows, 1, { hashHex: 'b2', faceIndex: 0 });
    expect(selectedFaces(rows).map(k => k.hashHex)).toEqual(['aa', 'b2']);
    rows = toggle(rows, 1);
    expect(selectedFaces(rows).map(k => k.hashHex)).toEqual(['aa']);
    rows = toggle(rows, 2); // family_only: không bật được
    expect(rows[2].include).toBe(false);
  });
  it('không thêm trùng một face khi hai dòng Figma cùng khớp một font', () => {
    const two: FigmaImportDto = { file: null, items: [imp.items[0], imp.items[0]] };
    const rows = buildRows(two, [matches[0], { ...matches[0], index: 1 }]);
    expect(selectedFaces(rows)).toHaveLength(1);
  });
});

describe('mock parse (cùng hợp đồng với backend)', () => {
  const link = (j: string) => `fontatlas://fonts?v=1&d=${btoa(j).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')}`;
  const J = '{"v":1,"file":"X","fonts":[["Inter","Regular",1]]}';
  it('JSON và link cho cùng kết quả; lỗi có mã', () => {
    expect(parseImportText(J)).toEqual(parseImportText(link(J)));
    expect(() => parseImportText('')).toThrowError(expect.objectContaining({ code: 'import_empty' }));
    expect(() => parseImportText('{"v":2,"fonts":[]}')).toThrowError(expect.objectContaining({ code: 'import_version' }));
    expect(() => parseImportText('xx')).toThrowError(expect.objectContaining({ code: 'import_format' }));
  });
});
