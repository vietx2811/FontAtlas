import type { CompareDto } from '../../api/compare';
import { COMPARE_METRICS } from '../../api/compare';
import { t, type I18nKey } from '../../i18n';
import { numberText, unitText } from '../detail/tabs/measurements';
import { classLabel, subclassLabel, traitLabel } from '../main/classLabels';
import { deltaOf, deltaText, fitScale, percentLabel, xhOk } from './compareMath';
import { OVERLAY_SPACED, OVERLAY_TEXT, SAMPLE_VI, viNotes } from './CompareCells';

export type ExportInput = { data: CompareDto; families: ReadonlyMap<string, string | null>; sample: string; fit: boolean; dark: boolean; overlay?: boolean };
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
export async function ensureFonts(families: Iterable<string | null>, sample: string): Promise<void> {
  if (typeof document === 'undefined' || !document.fonts?.load) return;
  const list = [...new Set([...families].filter((f): f is string => !!f))];
  const timeout = new Promise<void>(r => setTimeout(r, 4000));
  await Promise.race([Promise.all(list.flatMap(f => [document.fonts.load(`28px "${f}"`, sample), document.fonts.load(`30px "${f}"`, SAMPLE_VI)])).then(() => undefined), timeout]);
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
  dark: { bg: '#0C0A09', fg: '#F5F5F4', mute: '#A8A29E', faint: '#7C756F', line: '#2A2623', accent: '#E0B07E', page: '#0A0908', ink: '#FAFAF9', warn: '#FB923C', ok: '#4ADE80', ghost: 0.42, over: '#F0B777', rule: '#57534E' },
  light: { bg: '#FAFAFA', fg: '#2A2A2A', mute: '#78716C', faint: '#78716C', line: '#E7E5E4', accent: '#9A6B3C', page: '#FFFFFF', ink: '#1F1D1B', warn: '#C2410C', ok: '#3F7F5F', ghost: 0.28, over: '#9A6B3C', rule: '#D6D3D1' },
} as const;

export type ExportManifest = { rows: string[]; texts: string[]; size: { w: number; h: number }; dark: boolean; specimenBg: string };
export type ExportResult = { canvas: HTMLCanvasElement; manifest: ExportManifest };

/** Số đo kèm đơn vị, cùng hàm với màn hình (numberText + unitText). */
export const metricText = (value: number, unit: string | null): string => [numberText(value, unit), unitText(unit)].filter(Boolean).join(' ');

/** Vẽ bảng so sánh ra canvas (như màn hình: dòng mẫu, overlay, dấu chồng + nhãn, số đo + đơn vị + Δ, phân loại, license). Trả null khi không có canvas 2D (jsdom). */
export function renderCompare({ data, families, sample, fit, dark, overlay = true }: ExportInput): ExportResult | null {
  const n = data.faces.length;
  const LW = 200, CW = 270, PAD = 20, HEAD_LINE = 18, SPEC = 70, OVL = 96, LH = 15;
  const c = dark ? EXPORT_PALETTE.dark : EXPORT_PALETTE.light;
  const W = PAD * 2 + LW + n * CW;
  const ui = (typeof getComputedStyle === 'function' ? getComputedStyle(document.body).fontFamily : '') || 'sans-serif';
  const dpr = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
  const base = data.faces[0];
  const x0 = (i: number) => PAD + LW + i * CW;
  const fontOf = (size: number, weight = 400) => `${weight} ${size}px ${ui}`;
  const colW = CW - 16;

  // Đo trước để biết chiều cao: canvas tạm cùng font.
  const probe = document.createElement('canvas').getContext('2d');
  if (!probe) return null;
  const lines = (s: string, size: number, weight: number, maxW: number) => { probe.font = fontOf(size, weight); return wrapText(probe, s, maxW); };

  const names = data.faces.map((f, i) => `${i === 0 ? `(${t('compare.base')}) ` : ''}${f.row.family}${f.row.subfamily ? ' ' + f.row.subfamily : ''}`);
  const nameL = names.map(s => lines(s, 14, 600, colW));
  const headH = Math.max(...nameL.map(l => l.length)) * HEAD_LINE + 14;
  const labelBlock = (b: string, g: string) => ({ b: lines(b, 12, 600, LW - 12), g: g ? lines(g, 11, 400, LW - 12) : [] });
  const labelH = (b: string, g: string) => { const l = labelBlock(b, g); return (l.b.length + l.g.length) * LH + 18; };
  const viNotesOf = data.faces.map(f => viNotes(f).map(x => ({ ...x, lines: lines(x.text, 11, 600, colW) })));
  const viH = SPEC + Math.max(0, ...viNotesOf.map(a => a.reduce((h, x) => h + x.lines.length * 14 + 4, 0)));
  const metricK = (k: string) => `compare.m.${k}` as I18nKey;
  const deltaWraps = (k: (typeof COMPARE_METRICS)[number], i: number): { text: string; wrap: boolean } | null => {
    const m = data.faces[i].metrics[k];
    if (m.value === null || i === 0) return null;
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
    const own = labelH(t(metricK(k)), t(`${metricK(k)}.gloss` as I18nKey));
    const cells = Math.max(54, ...data.faces.map((f, i) => (deltaWraps(k, i)?.wrap ? 76 : 54) + (f.metrics[k].value === null ? 0 : 0)));
    return Math.max(own, cells);
  });
  const clsL = data.faces.map(f => {
    const k = f.classification;
    return lines(`${classLabel(k.class)}${k.subclass ? ' · ' + subclassLabel(k.subclass) : ''}${k.traits?.length ? ' · ' + k.traits.map(traitLabel).join(', ') : ''}`, 13, 600, colW);
  });
  const licL = data.faces.map(f => lines(`${t('compare.row.license')}: ${f.license ?? '—'} · ${t('compare.licenseNote')}`, 11, 400, colW));
  const clsH = Math.max(labelH(t('compare.row.class'), ''), ...data.faces.map((_, i) => clsL[i].length * 17 + 18 + licL[i].length * 14 + 12));

  const H = PAD * 2 + headH + SPEC + (overlay ? OVL : 0) + viH + metricH.reduce((a, b) => a + b, 0) + clsH;
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
  const fam = (i: number) => families.get(keyOf(data.faces[i].face)) ?? null;
  const scale = (i: number) => (fit && i > 0 ? fitScale(base.metrics.x_height_em?.value ?? null, data.faces[i].metrics.x_height_em?.value ?? null) : 1);
  const face = (i: number, px: number) => `${Math.round(px * scale(i))}px ${fam(i) ? `"${fam(i)}", ` : ''}${ui}`;

  band('specimen', t('compare.row.line'), '', SPEC, (i, bx, by) => {
    ctx.font = face(i, 28); ctx.fillStyle = c.ink; ctx.fillText(clipGraphemes(sample, 18), bx, by + 42);
    if (fit && i > 0 && !xhOk(data.faces[i].metrics.x_height_em?.value)) text(t('compare.noXh'), bx, by + 62, { size: 10, weight: 600, color: c.warn });
  });

  if (overlay) {
    const OSZ = 64, OY = 66;
    band('overlay', t('compare.row.overlay'), OVERLAY_SPACED, OVL, (i, bx, by, bw) => {
      const baseXh = base.metrics.x_height_em?.value;
      const same = i === 0;
      ctx.lineWidth = 1;
      ctx.strokeStyle = c.over; ctx.beginPath(); ctx.moveTo(bx - 10, by + OY); ctx.lineTo(bx + bw + 10, by + OY); ctx.stroke();
      if (xhOk(baseXh)) { ctx.strokeStyle = c.rule; ctx.setLineDash([3, 3]); ctx.beginPath(); ctx.moveTo(bx - 10, by + OY - baseXh! * OSZ); ctx.lineTo(bx + bw + 10, by + OY - baseXh! * OSZ); ctx.stroke(); ctx.setLineDash([]); }
      if (!same) { ctx.font = `${OSZ}px ${fam(0) ? `"${fam(0)}", ` : ''}${ui}`; ctx.globalAlpha = c.ghost; ctx.fillStyle = c.ink; ctx.fillText(OVERLAY_TEXT, bx - 2, by + OY); ctx.globalAlpha = 1; }
      ctx.font = `${OSZ * scale(i)}px ${fam(i) ? `"${fam(i)}", ` : ''}${ui}`; ctx.fillStyle = same ? c.ink : c.over; ctx.globalAlpha = same ? 1 : 0.9; ctx.fillText(OVERLAY_TEXT, bx - 2, by + OY); ctx.globalAlpha = 1;
      texts.push(OVERLAY_TEXT);
    });
  }

  band('vi', t('compare.row.vi'), 'Ữ Ặ Ệ Ỗ Ở · ươ', viH, (i, bx, by) => {
    const v = data.faces[i].vi;
    ctx.font = face(i, 30 * 1.1); ctx.fillStyle = c.ink; ctx.globalAlpha = v.supports ? 1 : 0.4; ctx.fillText(SAMPLE_VI, bx, by + 46); ctx.globalAlpha = 1;
    ctx.strokeStyle = v.stackedMarksOverflow ? c.warn : c.rule; ctx.setLineDash([3, 3]); ctx.beginPath(); ctx.moveTo(bx, by + 10); ctx.lineTo(bx + CW - 32, by + 10); ctx.stroke(); ctx.setLineDash([]);
    let ny = by + SPEC - 4;
    for (const note of viNotesOf[i]) { paras(note.lines, bx, ny + 10, 14, { size: 11, weight: 600, color: note.kind === 'ok' ? c.ok : c.warn }); ny += note.lines.length * 14 + 4; }
  });

  COMPARE_METRICS.forEach((k, ki) => {
    rows.push(`metric:${k}`);
    rule(y);
    rowHead(t(metricK(k)), t(`${metricK(k)}.gloss` as I18nKey), y);
    data.faces.forEach((f, i) => {
      const m = f.metrics[k]; const bx = x0(i);
      if (m.value === null) { text(t('compare.na'), bx, y + 28, { color: c.mute }); return; }
      const v = `${metricText(m.value, m.unit)}${m.quality === 'estimated' ? ' · ' + t('compare.est') : ''}`;
      text(v, bx, y + 24, { size: 15, weight: 600 });
      ctx.font = fontOf(15, 600); const vw = ctx.measureText(v).width;
      const d = deltaWraps(k, i);
      if (d) { if (d.wrap) text(d.text, bx, y + 42, { size: 11, weight: 600, color: c.accent }); else text(d.text, bx + vw + 8, y + 24, { size: 11, weight: 600, color: c.accent }); }
      else if (i === 0) text(t('compare.base'), bx + vw + 8, y + 24, { size: 11, color: c.faint });
      if (m.percentile !== null) {
        const by = y + (d?.wrap ? 56 : 38);
        ctx.fillStyle = c.line; ctx.fillRect(bx, by, CW - 40, 4);
        ctx.fillStyle = c.accent; ctx.fillRect(bx + (CW - 40) * Math.min(1, Math.max(0, m.percentile / 100)) - 1, by - 4, 2, 12);
        text(percentLabel(m.percentile), bx + CW - 36, by + 6, { size: 10, color: c.faint });
      }
    });
    y += metricH[ki];
  });

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
  return { canvas, manifest: { rows, texts, size: { w: W, h: H }, dark, specimenBg: c.page } };
}

/** Giữ chữ ký cũ: chỉ lấy canvas. */
export function renderCompareCanvas(input: ExportInput): HTMLCanvasElement | null { return renderCompare(input)?.canvas ?? null; }

/** Vẽ ra Blob PNG (đợi font trước). Ném lỗi nếu không có canvas 2D. */
export async function renderCompareBlob(input: ExportInput): Promise<{ blob: Blob; name: string }> {
  await ensureFonts(input.families.values(), input.sample);
  const canvas = renderCompareCanvas(input);
  if (!canvas) throw new Error('canvas 2D không khả dụng');
  const blob: Blob | null = await new Promise(res => canvas.toBlob(res, 'image/png'));
  if (!blob) throw new Error('không tạo được ảnh');
  return { blob, name: exportFileName(input.data) };
}
