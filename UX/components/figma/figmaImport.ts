// Logic thuần cho hộp thoại "Dán từ Figma": dựng dòng xem trước từ kết quả parse + resolve. Không gọi backend.
// Quy tắc: không đoán. Chỉ `exact` được chọn sẵn; `ambiguous` chờ người dùng chọn một ứng viên; `family_only` và `none` không vào Case.
import type { FaceKeyDto, FaceMatchDto, FigmaImportDto, FontMatchDto, MatchStatusDto } from '../../bindings';

export type ImportRow = {
  index: number; family: string; style: string; figmaMissing: boolean;
  status: MatchStatusDto; faces: FaceMatchDto[]; pick: FaceKeyDto | null; include: boolean;
};

const same = (a: FaceKeyDto, b: FaceKeyDto) => a.hashHex === b.hashHex && a.faceIndex === b.faceIndex;

export function buildRows(imp: FigmaImportDto, matches: FontMatchDto[]): ImportRow[] {
  return imp.items.map((it, i) => {
    const m = matches.find(x => x.index === i) ?? { index: i, status: 'none' as const, faces: [] };
    const exact = m.status === 'exact' && m.faces.length === 1;
    return { index: i, family: it.family, style: it.style, figmaMissing: it.figmaMissing, status: m.status, faces: m.faces, pick: exact ? m.faces[0].face : null, include: exact };
  });
}

/** Chọn (hoặc bỏ chọn bằng null) một ứng viên cho dòng `ambiguous`. Dòng khác không đổi. */
export function choose(rows: ImportRow[], index: number, face: FaceKeyDto | null): ImportRow[] {
  return rows.map(r => r.index !== index || r.status !== 'ambiguous' || (face && !r.faces.some(c => same(c.face, face))) ? r : { ...r, pick: face, include: face !== null });
}

/** Bật/tắt một dòng. Chỉ dòng đã có `pick` mới bật được. */
export function toggle(rows: ImportRow[], index: number): ImportRow[] {
  return rows.map(r => (r.index === index && r.pick ? { ...r, include: !r.include } : r));
}

export function selectedFaces(rows: ImportRow[]): FaceKeyDto[] {
  const out: FaceKeyDto[] = [];
  for (const r of rows) if (r.include && r.pick && !out.some(k => same(k, r.pick as FaceKeyDto))) out.push(r.pick);
  return out;
}

export function counts(rows: ImportRow[]) {
  const n = (s: MatchStatusDto) => rows.filter(r => r.status === s).length;
  return { total: rows.length, exact: n('exact'), ambiguous: n('ambiguous'), familyOnly: n('family_only'), none: n('none'), figmaMissing: rows.filter(r => r.figmaMissing).length };
}
