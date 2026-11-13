// Mock của nhập Figma (B6) cho dev không có Tauri và Vitest. Bám hợp đồng docs/team/plans/2026-11-13-resolve-fonts-contract.md.
import { CmdError } from './types';
import type { Api } from './index';
import type { FaceMatchDto, FaceRowDto, FigmaImportDto, MatchStatusDto } from '../bindings';

type FigmaApi = Pick<Api, 'parseFigmaImport' | 'resolveFonts' | 'takePendingImports' | 'figmaImportSubscribe'>;
const bad = (code: string) => new CmdError(code, code);
const fam = (s: string) => s.normalize('NFC').toLowerCase().split(/\s+/).filter(Boolean).join(' ');
const sty = (s: string) => s.normalize('NFC').toLowerCase().replace(/[\s_-]/g, '');

export function parseImportText(text: string): FigmaImportDto {
  if (text.length > 64 * 1024) throw bad('import_too_large');
  let raw = text.trim();
  if (!raw) throw bad('import_empty');
  if (raw.toLowerCase().startsWith('fontatlas://')) {
    const m = /^fontatlas:\/\/fonts\?(.*)$/i.exec(raw);
    const d = m && new URLSearchParams(m[1]).get('d');
    if (!d) throw bad('import_format');
    try { raw = decodeURIComponent(escape(atob(d.replace(/-/g, '+').replace(/_/g, '/')))); } catch { throw bad('import_format'); }
  }
  let v: { v?: unknown; file?: unknown; fonts?: unknown };
  try { v = JSON.parse(raw) as typeof v; } catch { throw bad('import_format'); }
  if (typeof v.v !== 'number') throw bad('import_format');
  if (v.v !== 1) throw bad('import_version');
  if (!Array.isArray(v.fonts)) throw bad('import_format');
  if (v.fonts.length > 200) throw bad('import_too_many_fonts');
  const items = v.fonts.map(f => {
    if (!Array.isArray(f) || typeof f[0] !== 'string' || typeof f[1] !== 'string' || !f[0].trim() || !f[1].trim()) throw bad('import_format');
    return { family: f[0].trim(), style: f[1].trim(), figmaMissing: f[2] === 1 || f[2] === true };
  });
  return { file: typeof v.file === 'string' && v.file.trim() ? v.file.trim().slice(0, 128) : null, items };
}

export function createMockFigma(rows: () => FaceRowDto[]): FigmaApi {
  return {
    async parseFigmaImport(text) { return parseImportText(text); },
    async resolveFonts(items) {
      if (items.length > 200) throw bad('import_too_many_fonts');
      return items.map((it, index) => {
        const same = rows().filter(r => fam(r.family) === fam(it.family));
        const hit = same.filter(r => sty(r.subfamily ?? '') === sty(it.style));
        const face = (r: FaceRowDto): FaceMatchDto => ({ face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, family: r.family, style: r.subfamily ?? '', missing: r.missing });
        const status: MatchStatusDto = !same.length ? 'none' : hit.length === 0 ? 'family_only' : hit.length === 1 ? 'exact' : 'ambiguous';
        return { index, status, faces: (hit.length ? hit : same).slice(0, 5).map(face) };
      });
    },
    async takePendingImports() { return []; },
    async figmaImportSubscribe() { return () => undefined; },
  };
}
