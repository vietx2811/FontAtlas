import type { CompareDto } from '../../api/compare';
import { COMPARE_METRICS } from '../../api/compare';
import { t } from '../../i18n';
import { numberText } from '../detail/tabs/measurements';
import { classLabel, subclassLabel, traitLabel } from '../main/classLabels';
import { deltaOf, deltaText, fitScale, percentLabel } from './compareMath';
import { SAMPLE_VI } from './CompareCells';

export type ExportInput = { data: CompareDto; families: ReadonlyMap<string, string | null>; sample: string; fit: boolean; dark: boolean };
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

/** Vẽ bảng so sánh ra canvas (chữ + thanh percentile + dòng mẫu bằng font thật). Trả null khi không có canvas 2D (jsdom). */
export function renderCompareCanvas({ data, families, sample, fit, dark }: ExportInput): HTMLCanvasElement | null {
  const n = data.faces.length;
  const LW = 150, CW = 260, PAD = 20, ROW = 54, HEAD = 56, SPEC = 70;
  const W = PAD * 2 + LW + n * CW, H = PAD * 2 + HEAD + SPEC * 2 + COMPARE_METRICS.length * ROW + 90;
  const dpr = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.scale(dpr, dpr);
  const ui = (typeof getComputedStyle === 'function' ? getComputedStyle(document.body).fontFamily : '') || 'sans-serif';
  const c = dark ? { bg: '#1C1917', fg: '#F5F5F4', mute: '#A8A29E', line: '#2E2A27', accent: '#E0B07E', page: '#F7F5F2', ink: '#1F1D1B', warn: '#C2410C' } : { bg: '#FAFAFA', fg: '#2A2A2A', mute: '#78716C', line: '#E7E5E4', accent: '#9A6B3C', page: '#FFFFFF', ink: '#1F1D1B', warn: '#C2410C' };
  ctx.fillStyle = c.bg; ctx.fillRect(0, 0, W, H);
  const base = data.faces[0];
  const x0 = (i: number) => PAD + LW + i * CW;
  const text = (s: string, x: number, y: number, o: { size?: number; weight?: number; color?: string; maxW?: number } = {}) => {
    ctx.font = `${o.weight ?? 400} ${o.size ?? 13}px ${ui}`; ctx.fillStyle = o.color ?? c.fg;
    ctx.fillText(o.maxW ? fitText(ctx, s, o.maxW) : s, x, y);
  };
  let y = PAD;
  data.faces.forEach((f, i) => text(`${i === 0 ? `(${t('compare.base')}) ` : ''}${f.row.family}${f.row.subfamily ? ' ' + f.row.subfamily : ''}`, x0(i), y + 20, { size: 14, weight: 600, maxW: CW - 16 }));
  y += HEAD;
  const band = (label: string, draw: (i: number, bx: number, by: number) => void) => {
    text(label, PAD, y + 40, { color: c.mute, maxW: LW - 8 });
    data.faces.forEach((_, i) => { ctx.save(); ctx.beginPath(); ctx.rect(x0(i), y, CW - 12, SPEC - 8); ctx.clip(); ctx.fillStyle = c.page; ctx.fillRect(x0(i), y, CW - 12, SPEC - 8); draw(i, x0(i) + 10, y); ctx.restore(); });
    y += SPEC;
  };
  const fam = (i: number) => families.get(keyOf(data.faces[i].face)) ?? null;
  const scale = (i: number) => (fit ? fitScale(base.metrics.x_height_em?.value ?? null, data.faces[i].metrics.x_height_em?.value ?? null) : 1);
  const face = (i: number, px: number) => `${Math.round(px * scale(i))}px ${fam(i) ? `"${fam(i)}", ` : ''}${ui}`;
  band(t('compare.row.line'), (i, bx, by) => { ctx.font = face(i, 28); ctx.fillStyle = c.ink; ctx.fillText(clipGraphemes(sample, 18), bx, by + 42); });
  band(t('compare.row.vi'), (i, bx, by) => { const v = data.faces[i].vi; ctx.font = face(i, 30); ctx.fillStyle = v.stackedMarksOverflow ? c.warn : c.ink; ctx.globalAlpha = v.supports ? 1 : 0.4; ctx.fillText(SAMPLE_VI, bx, by + 46); ctx.globalAlpha = 1; });
  for (const k of COMPARE_METRICS) {
    ctx.strokeStyle = c.line; ctx.beginPath(); ctx.moveTo(PAD, y); ctx.lineTo(W - PAD, y); ctx.stroke();
    text(t(`compare.m.${k}` as 'compare.m.contrast'), PAD, y + 24, { size: 12, weight: 600, maxW: LW - 8 });
    text(t(`compare.m.${k}.gloss` as 'compare.m.contrast'), PAD, y + 40, { size: 11, color: c.mute, maxW: LW - 8 });
    data.faces.forEach((f, i) => {
      const m = f.metrics[k]; const bx = x0(i);
      if (m.value === null) { text(t('compare.na'), bx, y + 28, { color: c.mute }); return; }
      text(`${numberText(m.value, m.unit)}${m.quality === 'estimated' ? ' · ' + t('compare.est') : ''}`, bx, y + 24, { size: 15, weight: 600, maxW: 130 });
      const d = i === 0 ? null : deltaOf(k, m, base.metrics[k]);
      if (d) text(deltaText(d).text, bx + 130, y + 24, { size: 11, weight: 600, color: c.accent });
      if (m.percentile !== null) {
        ctx.fillStyle = c.line; ctx.fillRect(bx, y + 36, CW - 40, 4);
        ctx.fillStyle = c.accent; ctx.fillRect(bx + (CW - 40) * Math.min(1, Math.max(0, m.percentile / 100)) - 1, y + 32, 2, 12);
        text(percentLabel(m.percentile), bx + CW - 36, y + 42, { size: 10, color: c.mute });
      }
    });
    y += ROW;
  }
  ctx.strokeStyle = c.line; ctx.beginPath(); ctx.moveTo(PAD, y); ctx.lineTo(W - PAD, y); ctx.stroke();
  text(t('compare.row.class'), PAD, y + 28, { size: 12, weight: 600, maxW: LW - 8 });
  data.faces.forEach((f, i) => {
    const k = f.classification;
    const conf = Number.isFinite(k.confidence) ? `${Math.round(k.confidence * 100)}%` : '';
    text(`${classLabel(k.class)}${k.subclass ? ' · ' + subclassLabel(k.subclass) : ''}${k.traits?.length ? ' · ' + k.traits.map(traitLabel).join(', ') : ''}`, x0(i), y + 28, { size: 13, weight: 600, maxW: CW - 16 });
    if (conf) text(conf, x0(i), y + 46, { size: 11, color: c.mute });
    text(`${t('compare.row.license')}: ${f.license ?? '—'} · ${t('compare.licenseNote')}`, x0(i), y + 66, { size: 11, color: c.mute, maxW: CW - 16 });
  });
  return canvas;
}

/** Vẽ ra Blob PNG (đợi font trước). Ném lỗi nếu không có canvas 2D. */
export async function renderCompareBlob(input: ExportInput): Promise<{ blob: Blob; name: string }> {
  await ensureFonts(input.families.values(), input.sample);
  const canvas = renderCompareCanvas(input);
  if (!canvas) throw new Error('canvas 2D không khả dụng');
  const blob: Blob | null = await new Promise(res => canvas.toBlob(res, 'image/png'));
  if (!blob) throw new Error('không tạo được ảnh');
  return { blob, name: exportFileName(input.data) };
}
