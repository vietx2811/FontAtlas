import type { Api, FaceKeyDto } from '../../api';
import type { CompareDto } from '../../api/compare';
import { COMPARE_METRICS } from '../../api/compare';
import { t, type I18nKey } from '../../i18n';
import { numberText, unitText } from '../detail/tabs/measurements';
import { classLabel, subclassLabel, traitLabel } from '../main/classLabels';
import { barFill, barScale, deltaOf, deltaText, fitScale, percentLabel, xhOk } from './compareMath';
import { measureInkWith } from '../ui/inkLines';
import { termName } from '../ui/termNames';
import { OVERLAY_SPACED, OVERLAY_TEXT, SAMPLE_VI, faceName, overlayStyleOf, viNotes } from './CompareCells';
import { coverageProbe, graphemes, missingList } from './glyphCoverage';

export type ExportInput = { loaded?: ReadonlySet<string>; data: CompareDto; families: ReadonlyMap<string, string | null>; sample: string; fit: boolean; dark: boolean; /** Khoá face (hashHex:faceIndex) được chồng trong Overlay, theo toggle trên màn hình; null/rỗng = không vẽ vùng Overlay. */ overlay?: readonly string[] | null; /** Thứ tự cột như màn hình (khoá face, theo pin); mốc vẫn là cột đầu của `data`. */ order?: readonly string[]; /** Hàng ẩn (id như màn hình, vd `metrics.x_height_em`) và nhóm thu gọn (`specimen`/`metrics`/`class`) → không vẽ. */ hiddenRows?: ReadonlySet<string>; collapsed?: ReadonlySet<string>; /** Họ font → ký tự của mẫu mà font đó không có (xám + ghi chú chữ, vì ảnh không có hover). */ missing?: ReadonlyMap<string, ReadonlySet<string>> };
const keyOf = (f: { hashHex: string; faceIndex: number }) => `${f.hashHex}:${f.faceIndex}`;

export const exportFileName = (data: CompareDto): string =>
  `compare-${data.faces.map(f => f.row.family.replace(/[^\p{L}\p{N}]+/gu, '-')).join('_')}.png`.slice(0, 120);

/** Cắt theo grapheme (không chẻ đôi chữ + dấu) tối đa n ký tự. */
export function clipGraphemes(s: string, n: number): string {
  const seg = (Intl as { Segmenter?: new (l?: string, o?: object) => { segment(s: string): Iterable<{ segment: string }> } }).Segmenter;
  const parts = seg ? [...new seg('vi', { granularity: 'grapheme' }).segment(s)].map(x => x.segment) : [...s.normalize('NFC')];
  return parts.length <= n ? s : parts.slice(0, n).join('');
}
/** Rút gọn bằng "…" cho vừa bề ngang. */
export function fitText(ctx: Pick<CanvasRenderingContext2D, 'measureText'>, text: string, maxW: number): string {
  if (ctx.measureText(text).width <= maxW) return text;
  let lo = 0, hi = text.length;
  while (lo < hi) { const mid = Math.ceil((lo + hi) / 2); if (ctx.measureText(text.slice(0, mid) + '…').width <= maxW) lo = mid; else hi = mid - 1; }
  return text.slice(0, lo) + '…';
}

/** Chờ các font cột nạp xong (có hạn chờ) để canvas không vẽ bằng font dự phòng. */
export async function ensureFonts(families: Iterable<string | null>, sample: string): Promise<Set<string>> {
  const loaded = new Set<string>();
  if (typeof document === 'undefined' || !document.fonts?.load) return loaded;
  const list = [...new Set([...families].filter((f): f is string => !!f))];
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<void>(r => { timer = setTimeout(r, 4000); });
  // Một font reject không làm hỏng cả lần xuất: allSettled, font lỗi vẽ bằng font dự phòng như nhánh hết giờ.
  const one = async (f: string) => {
    const r = await Promise.allSettled([document.fonts.load(`28px "${f}"`, sample), document.fonts.load(`30px "${f}"`, SAMPLE_VI), document.fonts.load(`64px "${f}"`, OVERLAY_TEXT)]);
    if (r.some(x => x.status === 'fulfilled' && (x.value as unknown[]).length > 0)) loaded.add(f);
  };
  try { await Promise.race([Promise.all(list.map(one)).then(() => undefined), timeout]); } finally { clearTimeout(timer); }
  return loaded;
}

/** Họ font → grapheme của `text` mà font đó không có (cmap thật qua backend); dùng cho chữ xám + ghi chú trong ảnh. */
export async function detectMissingAll(api: Api, faces: ReadonlyArray<{ face: FaceKeyDto; family: string | null }>, text: string): Promise<Map<string, Set<string>>> {
  const out = new Map<string, Set<string>>();
  await Promise.all(faces.map(async ({ face, family }) => { if (!family) return; const m = await coverageProbe.detect(api, face, text); if (m && m.size) out.set(family, m); }));
  return out;
}

type Measure = Pick<CanvasRenderingContext2D, 'measureText'>;
/** Ngắt dòng theo từ (từ dài hơn bề ngang thì ngắt theo ký tự); không bao giờ dùng "…". */
export function wrapText(ctx: Measure, text: string, maxW: number): string[] {
  const out: string[] = [];
  let line = '';
  const push = (w: string) => {
    const next = line ? `${line} ${w}` : w;
    if (ctx.measureText(next).width <= maxW) { line = next; return; }
    if (line) { out.push(line); line = ''; }
    if (ctx.measureText(w).width <= maxW) { line = w; return; }
    let cur = '';
    for (const ch of w) { if (cur && ctx.measureText(cur + ch).width > maxW) { out.push(cur); cur = ch; } else cur += ch; }
    line = cur;
  };
  for (const w of text.split(/\s+/).filter(Boolean)) push(w);
  if (line) out.push(line);
  return out.length ? out : [''];
}

/** Bảng màu ảnh theo theme. Specimen: nền/chữ khớp --specimen-* trong index.css (sáng: trắng/đen ấm; tối: gần đen/trắng). */
export const EXPORT_PALETTE = {
  dark: { bg: '#0C0A09', fg: '#F5F5F4', mute: '#A8A29E', faint: '#8A837D', line: '#2A2623', accent: '#E0B07E', page: '#0A0908', ink: '#FAFAF9', warn: '#FB923C', ok: '#4ADE80', ghost: 0.42, over: '#F0B777', rule: '#57534E' },
  light: { bg: '#FAFAFA', fg: '#2A2A2A', mute: '#78716C', faint: '#78716C', line: '#E7E5E4', accent: '#9A6B3C', page: '#FFFFFF', ink: '#1F1D1B', warn: '#C2410C', ok: '#3F7F5F', ghost: 0.28, over: '#9A6B3C', rule: '#D6D3D1' },
} as const;

export type ExportManifest = { rows: string[]; texts: string[]; fit: { specimen: number; vi: number }; size: { w: number; h: number }; dark: boolean; specimenBg: string; overlayLayers: number };
export type ExportResult = { canvas: HTMLCanvasElement; manifest: ExportManifest };

/** Số đo kèm đơn vị, cùng hàm với màn hình (numberText + unitText). */
export const metricText = (value: number, unit: string | null): string => [numberText(value, unit), unitText(unit)].filter(Boolean).join(' ');

/** Vẽ bảng so sánh ra canvas (như màn hình: dòng mẫu, overlay, dấu chồng + nhãn, số đo + đơn vị + Δ, phân loại, license). Trả null khi không có canvas 2D (jsdom). */
export function renderCompare({ data: source, families, sample, fit, dark, overlay, loaded, missing, order, hiddenRows, collapsed }: ExportInput): ExportResult | null {
  const base = source.faces[0];
  const picked = order ? order.map(k => source.faces.find(f => keyOf(f.face) === k)).filter((f): f is NonNullable<typeof f> => !!f) : source.faces;
  const data: CompareDto = { ...source, faces: picked.length === source.faces.length ? picked : source.faces };
  const shows = (id: string, group: string): boolean => !hiddenRows?.has(id) && !collapsed?.has(group);
  const n = data.faces.length;
  const LW = 200, CW = 270, PAD = 20, HEAD_LINE = 18, SPEC = 70, OVL = 96, LH = 15;
  const c = dark ? EXPORT_PALETTE.dark : EXPORT_PALETTE.light;
  const W = PAD * 2 + LW + n * CW;
  const ui = (typeof getComputedStyle === 'function' ? getComputedStyle(document.body).fontFamily : '') || 'sans-serif';
  const dpr = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
  const x0 = (i: number) => PAD + LW + i * CW;
  const fontOf = (size: number, weight = 400) => `${weight} ${size}px ${ui}`;
  const colW = CW - 16;

  // Đo trước để biết chiều cao: canvas tạm cùng font.
  const probe = document.createElement('canvas').getContext('2d');
  if (!probe) return null;
  const lines = (s: string, size: number, weight: number, maxW: number) => { probe.font = fontOf(size, weight); return wrapText(probe, s, maxW); };

  const names = data.faces.map((f, i) => `${f === base ? `(${t('compare.base')}) ` : ''}${f.row.family}${f.row.subfamily ? ' ' + f.row.subfamily : ''}`);
  const nameL = names.map(s => lines(s, 14, 600, colW));
  const headH = Math.max(...nameL.map(l => l.length)) * HEAD_LINE + 14;
  const labelBlock = (b: string, g: string) => ({ b: lines(b, 12, 600, LW - 12), g: g ? lines(g, 11, 400, LW - 12) : [] });
  const labelH = (b: string, g: string) => { const l = labelBlock(b, g); return (l.b.length + l.g.length) * LH + 18; };
  const fam = (i: number) => { const f = families.get(keyOf(data.faces[i].face)) ?? null; return f && (!loaded || loaded.has(f)) ? f : null; };
  const missOf = (i: number, str: string): Set<string> => { const f = fam(i); const set = f ? missing?.get(f) : undefined; return new Set(set ? graphemes(str).filter(ch => set.has(ch)) : []); };
  const specText = clipGraphemes(sample || 'Hxg', 18);
  const viNotesOf = data.faces.map((f, i) => {
    const m = missOf(i, SAMPLE_VI);
    const all = m.size ? [...viNotes(f), { kind: 'warn' as const, text: t('compare.missing', { chars: missingList(m) }) }] : viNotes(f);
    return all.map(x => ({ ...x, lines: lines(x.text, 11, 600, colW) }));
  });
  const specNotes = (i: number): string[] => [...(fit && data.faces[i] !== base && !xhOk(data.faces[i].metrics.x_height_em?.value) ? [t('compare.noXh')] : []), ...(missOf(i, specText).size ? [t('compare.missing', { chars: missingList(missOf(i, specText)) })] : [])];
  const maxSpecNotes = Math.max(0, ...data.faces.map((_, i) => specNotes(i).length));
  // Chiều cao hàng không thấp hơn khối nhãn bên trái sau khi xuống dòng.
  const viGloss = `${termName('vi_marks')} · ${t('compare.row.vi.gloss')}`;
  const specH = !shows('specimen.line', 'specimen') ? 0 : Math.max(SPEC + 14 * maxSpecNotes, labelH(t('compare.row.line'), ''));
  // Vùng Overlay chung (một khối trải hết các cột): các font được chọn chồng lên nhau + chú giải tên.
  const ovSel = data.faces.filter(f => overlay?.includes(keyOf(f.face)));
  const ovOn = ovSel.length > 0;
  const ovStyles = overlayStyleOf(ovSel, base);
  const legendW = n * CW - 12 - 20;
  const legend = (() => {
    const rowsL: Array<Array<{ f: (typeof ovSel)[number]; x: number }>> = [[]]; let x = 0;
    probe.font = fontOf(11, 400);
    for (const f of ovSel) {
      const w = 34 + probe.measureText(`${faceName(f)}${f === base ? ` · ${t('compare.base')}` : ''}`).width + 18;
      if (x + w > legendW && rowsL.at(-1)!.length) { rowsL.push([]); x = 0; }
      rowsL.at(-1)!.push({ f, x }); x += w;
    }
    return rowsL;
  })();
  const ovlH = Math.max(OVL + legend.length * 16, labelH(t('compare.row.overlay'), OVERLAY_SPACED));
  const viH = !shows('specimen.vi', 'specimen') ? 0 : Math.max(SPEC + Math.max(0, ...viNotesOf.map(a => a.reduce((h, x) => h + x.lines.length * 14 + 4, 0))), labelH(t('compare.row.vi'), viGloss));
  const metricK = (k: string) => `compare.m.${k}` as I18nKey;
  const deltaWraps = (k: (typeof COMPARE_METRICS)[number], i: number): { text: string; wrap: boolean } | null => {
    const m = data.faces[i].metrics[k];
    if (m.value === null || data.faces[i] === base) return null;
    const d = deltaOf(k, m, base.metrics[k]);
    if (!d) return null;
    const text = deltaText(d).text;
    probe.font = fontOf(15, 600);
    const v = `${metricText(m.value, m.unit)}${m.quality === 'estimated' ? ' · ' + t('compare.est') : ''}`;
    const vw = probe.measureText(v).width;
    probe.font = fontOf(11, 600);
    return { text, wrap: vw + 8 + probe.measureText(text).width > colW };
  };
  const metricH = COMPARE_METRICS.map(k => {
    if (!shows(`metrics.${k}`, 'metrics')) return 0;
    const own = labelH(termName(k), t(`${metricK(k)}.gloss` as I18nKey));
    const cells = Math.max(54, ...data.faces.map((f, i) => (deltaWraps(k, i)?.wrap ? 76 : 54) + (f.metrics[k].value === null ? 0 : 0)));
    return Math.max(own, cells);
  });
  const clsL = data.faces.map(f => {
    const k = f.classification;
    return lines(`${classLabel(k.class)}${k.subclass ? ' · ' + subclassLabel(k.subclass) : ''}${k.traits?.length ? ' · ' + k.traits.map(traitLabel).join(', ') : ''}`, 13, 600, colW);
  });
  const licL = data.faces.map(f => lines(`${t('compare.row.license')}: ${f.license ?? '—'} · ${t('compare.licenseNote')}`, 11, 400, colW));
  const clsH = !shows('class.class', 'class') ? 0 : Math.max(labelH(t('compare.row.class'), ''), ...data.faces.map((_, i) => clsL[i].length * 17 + 18 + licL[i].length * 14 + 12));

  const H = PAD * 2 + headH + specH + (ovOn ? ovlH : 0) + viH + metricH.reduce((a, b) => a + b, 0) + clsH;
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.scale(dpr, dpr);
  const rows: string[] = []; const texts: string[] = [];
  ctx.fillStyle = c.bg; ctx.fillRect(0, 0, W, H);
  const text = (s: string, x: number, y: number, o: { size?: number; weight?: number; color?: string } = {}) => {
    ctx.font = fontOf(o.size ?? 13, o.weight ?? 400); ctx.fillStyle = o.color ?? c.fg; ctx.fillText(s, x, y); texts.push(s);
  };
  const paras = (ls: string[], x: number, y: number, step: number, o: { size?: number; weight?: number; color?: string }) => ls.forEach((l, j) => text(l, x, y + j * step, o));
  const rowHead = (b: string, g: string, y: number) => { const l = labelBlock(b, g); paras(l.b, PAD, y + 20, LH, { size: 12, weight: 600 }); paras(l.g, PAD, y + 20 + l.b.length * LH, LH, { size: 11, color: c.mute }); };
  const rule = (y: number) => { ctx.strokeStyle = c.line; ctx.beginPath(); ctx.moveTo(PAD, y); ctx.lineTo(W - PAD, y); ctx.stroke(); };

  let y = PAD;
  rows.push('header');
  nameL.forEach((l, i) => paras(l, x0(i), y + 18, HEAD_LINE, { size: 14, weight: 600 }));
  y += headH;

  const band = (kind: string, label: string, gloss: string, h: number, draw: (i: number, bx: number, by: number, bw: number, bh: number) => void) => {
    rows.push(kind);
    rowHead(label, gloss, y);
    data.faces.forEach((_, i) => { ctx.save(); ctx.beginPath(); ctx.rect(x0(i), y, CW - 12, h - 8); ctx.clip(); ctx.fillStyle = c.page; ctx.fillRect(x0(i), y, CW - 12, h - 8); draw(i, x0(i) + 10, y, CW - 32, h - 8); ctx.restore(); });
    y += h;
  };
  const scale = (i: number) => (fit && data.faces[i] !== base ? fitScale(base.metrics.x_height_em?.value ?? null, data.faces[i].metrics.x_height_em?.value ?? null) : 1);
  const face = (i: number, px: number) => `${Math.round(px * scale(i))}px ${fam(i) ? `"${fam(i)}", ` : ''}${ui}`;

  // Chữ không bao giờ bị cắt ở mép cột: một hệ số chung cho cả hàng (giữ tỷ lệ giữa các font trong cùng chế độ Same size / Match x-height).
  const fitK = (str: string, px: number): number => {
    let k = 1;
    for (let i = 0; i < n; i++) { ctx.font = face(i, px); const w = ctx.measureText(str).width; if (w > 0) k = Math.min(k, (CW - 36) / w); }
    return k;
  };
  const kSpec = fitK(specText, 28); const kVi = fitK(SAMPLE_VI, 30 * 1.1);
  // Ký tự thiếu vẽ màu xám (--faint tương ứng); ghi chú chữ nằm dưới mẫu vì ảnh không có hover.
  const fillMarked = (str: string, x: number, yy: number, miss: Set<string>) => {
    if (miss.size === 0) { ctx.fillStyle = c.ink; ctx.fillText(str, x, yy); return; }
    let acc = '';
    for (const ch of graphemes(str)) { ctx.fillStyle = miss.has(ch) ? c.faint : c.ink; ctx.fillText(ch, x + ctx.measureText(acc).width, yy); acc += ch; }
  };
  if (specH > 0) band('specimen', t('compare.row.line'), '', specH, (i, bx, by) => {
    ctx.font = face(i, 28 * kSpec); fillMarked(specText, bx, by + 42, missOf(i, specText));
    specNotes(i).forEach((nt, j) => text(nt, bx, by + 62 + 14 * j, { size: 10, weight: 600, color: c.warn }));
  });

  if (ovOn) {
    const OSZ = 72, OY = 82;
    rows.push('overlay');
    rowHead(t('compare.row.overlay'), OVERLAY_SPACED, y);
    const bx = x0(0) + 10; const bw = n * CW - 12;
    ctx.save(); ctx.beginPath(); ctx.rect(x0(0), y, bw, ovlH - 8); ctx.clip(); ctx.fillStyle = c.page; ctx.fillRect(x0(0), y, bw, ovlH - 8);
    const idxOf = (f: (typeof ovSel)[number]) => data.faces.indexOf(f);
    const primary = ovSel.includes(base) ? base : ovSel[0];
    const pf = fam(idxOf(primary));
    const xh = pf ? measureInkWith(ctx, `${OSZ}px "${pf}"`, OSZ)?.x ?? null : null; // x-height đo từ chữ đang vẽ của font chính
    ctx.lineWidth = 1; ctx.strokeStyle = c.over; ctx.beginPath(); ctx.moveTo(bx - 10, y + OY); ctx.lineTo(Math.min(bx + 520, bx + bw - 20), y + OY); ctx.stroke();
    if (xh !== null) { ctx.strokeStyle = c.rule; ctx.setLineDash([3, 3]); ctx.beginPath(); ctx.moveTo(bx - 10, y + OY - xh * OSZ); ctx.lineTo(Math.min(bx + 520, bx + bw - 20), y + OY - xh * OSZ); ctx.stroke(); ctx.setLineDash([]); }
    const strokeOf = (st: string, g = ctx) => { g.lineWidth = st === 'dot' ? 1.5 : 1.3; g.globalAlpha = st === 'dash' ? 0.62 : st === 'dot' ? 0.4 : 1; g.setLineDash(st === 'dash' ? [5, 2.5] : st === 'dot' ? [1.5, 2.5] : []); };
    for (const f of ovSel) {
      const i = idxOf(f); const st = ovStyles.get(f)!;
      ctx.font = `${OSZ * scale(i)}px ${fam(i) ? `"${fam(i)}", ` : ''}${ui}`;
      if (st === 'solid') { ctx.fillStyle = c.ink; ctx.fillText(OVERLAY_TEXT, bx - 2, y + OY); }
      else { ctx.strokeStyle = c.ink; strokeOf(st); ctx.strokeText(OVERLAY_TEXT, bx - 2, y + OY); ctx.globalAlpha = 1; ctx.setLineDash([]); }
    }
    texts.push(OVERLAY_TEXT);
    legend.forEach((row, li) => row.forEach(({ f, x }) => {
      const ly = y + OVL - 6 + li * 16; const st = ovStyles.get(f)!;
      ctx.strokeStyle = c.ink; ctx.lineWidth = 2.4; strokeOf(st); ctx.lineWidth = 2.4;
      ctx.beginPath(); ctx.moveTo(bx + x, ly - 4); ctx.lineTo(bx + x + 26, ly - 4); ctx.stroke(); ctx.globalAlpha = 1; ctx.setLineDash([]);
      text(`${faceName(f)}${f === base ? ` · ${t('compare.base')}` : ''}`, bx + x + 34, ly, { size: 11, color: c.ink });
    }));
    ctx.restore();
    y += ovlH;
  }

  if (viH > 0) band('vi', t('compare.row.vi'), viGloss, viH, (i, bx, by) => {
    const v = data.faces[i].vi;
    ctx.font = face(i, 30 * 1.1 * kVi); ctx.globalAlpha = v.supports ? 1 : 0.4; fillMarked(SAMPLE_VI, bx, by + 46, missOf(i, SAMPLE_VI)); ctx.globalAlpha = 1;
    if (v.stackedMarksOverflow) { ctx.strokeStyle = c.rule; ctx.beginPath(); ctx.moveTo(bx, by + 10); ctx.lineTo(bx + CW - 32, by + 10); ctx.stroke(); } // đường xám nhạt chỉ khi dấu tràn; cảnh báo bằng chữ bên dưới
    let ny = by + SPEC - 4;
    for (const note of viNotesOf[i]) { paras(note.lines, bx, ny + 10, 14, { size: 11, weight: 600, color: note.kind === 'ok' ? c.ok : c.warn }); ny += note.lines.length * 14 + 4; }
  });

  COMPARE_METRICS.forEach((k, ki) => {
    if (metricH[ki] === 0) return;
    rows.push(`metric:${k}`);
    rule(y);
    rowHead(termName(k), t(`${metricK(k)}.gloss` as I18nKey), y);
    data.faces.forEach((f, i) => {
      const m = f.metrics[k]; const bx = x0(i);
      if (m.value === null) { text(t('compare.na'), bx, y + 28, { color: c.mute }); return; }
      const v = `${metricText(m.value, m.unit)}${m.quality === 'estimated' ? ' · ' + t('compare.est') : ''}`;
      text(v, bx, y + 24, { size: 15, weight: 600 });
      ctx.font = fontOf(15, 600); const vw = ctx.measureText(v).width;
      const d = deltaWraps(k, i);
      if (d) { if (d.wrap) text(d.text, bx, y + 42, { size: 11, weight: 600, color: c.accent }); else text(d.text, bx + vw + 8, y + 24, { size: 11, weight: 600, color: c.accent }); }
      else if (f === base) text(t('compare.base'), bx + vw + 8, y + 24, { size: 11, color: c.faint });
      // Thanh đo như màn hình: dương 0 → max×1,15, có dấu đối xứng quanh 0; tô từ 0. Percentile ghi bằng chữ vì ảnh không có tooltip.
      const range = barScale(k, data.faces.map(x => x.metrics[k].value));
      const by = y + (d?.wrap ? 56 : 38);
      if (range) {
        const f = barFill(m.value, range); const bw = CW - 40;
        ctx.fillStyle = c.line; ctx.fillRect(bx, by, bw, 4);
        ctx.fillStyle = c.accent; ctx.fillRect(bx + bw * f.left / 100, by, Math.max(2, bw * f.width / 100), 4);
        if (range.signed) { ctx.fillStyle = c.faint; ctx.fillRect(bx + bw / 2 - 0.5, by - 3, 1, 10); }
      }
      if (m.percentile !== null) text(percentLabel(m.percentile), range ? bx + CW - 36 : bx, by + 6, { size: 10, color: c.faint });
    });
    y += metricH[ki];
  });

  if (clsH > 0) {
    rows.push('class');
    rule(y);
    rowHead(t('compare.row.class'), '', y);
    data.faces.forEach((f, i) => {
      const k = f.classification;
      const conf = Number.isFinite(k.confidence) ? `${Math.round(k.confidence * 100)}%` : '';
      paras(clsL[i], x0(i), y + 22, 17, { size: 13, weight: 600 });
      let ly = y + 22 + clsL[i].length * 17;
      if (conf) { text(conf, x0(i), ly, { size: 11, color: c.mute }); ly += 18; }
      paras(licL[i], x0(i), ly + 4, 14, { size: 11, color: c.mute });
    });
  }
  return { canvas, manifest: { rows, texts, fit: { specimen: kSpec, vi: kVi }, size: { w: W, h: H }, dark, specimenBg: c.page, overlayLayers: ovSel.length } };
}

/** Giữ chữ ký cũ: chỉ lấy canvas. */
export function renderCompareCanvas(input: ExportInput): HTMLCanvasElement | null { return renderCompare(input)?.canvas ?? null; }

/** Vẽ ra Blob PNG (đợi font trước). Ném lỗi nếu không có canvas 2D. */
export async function renderCompareBlob(input: ExportInput): Promise<{ blob: Blob; name: string }> {
  const loaded = await ensureFonts(input.families.values(), input.sample);
  const canvas = renderCompareCanvas({ ...input, loaded });
  if (!canvas) throw new Error('canvas 2D không khả dụng');
  const blob: Blob | null = await new Promise(res => canvas.toBlob(res, 'image/png'));
  if (!blob) throw new Error('không tạo được ảnh');
  return { blob, name: exportFileName(input.data) };
}
