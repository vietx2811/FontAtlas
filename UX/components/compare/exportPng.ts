import type { CompareDto } from '../../api/compare';
import { COMPARE_METRICS } from '../../api/compare';
import { t } from '../../i18n';
import { numberText } from '../detail/tabs/measurements';
import { classLabel, subclassLabel } from '../main/classLabels';
import { deltaOf, deltaText, fitScale, percentLabel } from './compareMath';

export type ExportInput = { data: CompareDto; families: ReadonlyMap<string, string | null>; sample: string; fit: boolean; dark: boolean };
const keyOf = (f: { hashHex: string; faceIndex: number }) => `${f.hashHex}:${f.faceIndex}`;

export const exportFileName = (data: CompareDto): string =>
  `compare-${data.faces.map(f => f.row.family.replace(/[^\p{L}\p{N}]+/gu, '-')).join('_')}.png`.slice(0, 120);

/** Vẽ bảng so sánh ra canvas (chữ + thanh percentile + dòng mẫu bằng font thật). Trả null khi không có canvas 2D (jsdom). */
export function renderCompareCanvas({ data, families, sample, fit, dark }: ExportInput): HTMLCanvasElement | null {
  const n = data.faces.length;
  const LW = 150, CW = 260, PAD = 20, ROW = 54, HEAD = 56, SPEC = 70;
  const W = PAD * 2 + LW + n * CW, H = PAD * 2 + HEAD + SPEC * 2 + COMPARE_METRICS.length * ROW + 90;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  const c = dark ? { bg: '#1C1917', fg: '#F5F5F4', mute: '#A8A29E', line: '#2E2A27', accent: '#E0B07E', page: '#F7F5F2', ink: '#1F1D1B' } : { bg: '#FAFAFA', fg: '#2A2A2A', mute: '#78716C', line: '#E7E5E4', accent: '#9A6B3C', page: '#FFFFFF', ink: '#1F1D1B' };
  ctx.fillStyle = c.bg; ctx.fillRect(0, 0, W, H);
  const base = data.faces[0];
  const x0 = (i: number) => PAD + LW + i * CW;
  const text = (s: string, x: number, y: number, o: { font?: string; color?: string } = {}) => { ctx.font = o.font ?? '13px Inter, sans-serif'; ctx.fillStyle = o.color ?? c.fg; ctx.fillText(s, x, y); };
  let y = PAD;
  data.faces.forEach((f, i) => text(`${i === 0 ? '⚓ ' : ''}${f.row.family}${f.row.subfamily ? ' ' + f.row.subfamily : ''}`, x0(i), y + 20, { font: '600 14px Inter, sans-serif' }));
  y += HEAD;
  // Specimen + dấu chồng: nền trang sáng, chữ bằng font thật của cột.
  const band = (label: string, draw: (i: number, bx: number, by: number) => void) => {
    text(label, PAD, y + 40, { color: c.mute });
    data.faces.forEach((_, i) => { ctx.fillStyle = c.page; ctx.fillRect(x0(i), y, CW - 12, SPEC - 8); draw(i, x0(i) + 10, y); });
    y += SPEC;
  };
  const fam = (i: number) => families.get(keyOf(data.faces[i].face)) ?? null;
  const scale = (i: number) => (fit ? fitScale(base.metrics.x_height_em?.value ?? null, data.faces[i].metrics.x_height_em?.value ?? null) : 1);
  band(t('compare.row.line'), (i, bx, by) => { ctx.font = `${Math.round(28 * scale(i))}px ${fam(i) ? `"${fam(i)}"` : 'Inter'}, sans-serif`; ctx.fillStyle = c.ink; ctx.fillText(sample.slice(0, 18), bx, by + 42); });
  band(t('compare.row.vi'), (i, bx, by) => { ctx.font = `${Math.round(30 * scale(i))}px ${fam(i) ? `"${fam(i)}"` : 'Inter'}, sans-serif`; ctx.fillStyle = data.faces[i].vi.stackedMarksOverflow ? '#C2410C' : c.ink; ctx.fillText('ỮẶỆỖỞ ươ', bx, by + 46); });
  for (const k of COMPARE_METRICS) {
    ctx.strokeStyle = c.line; ctx.beginPath(); ctx.moveTo(PAD, y); ctx.lineTo(W - PAD, y); ctx.stroke();
    text(t(`compare.m.${k}` as 'compare.m.contrast'), PAD, y + 24, { font: '600 12px Inter, sans-serif' });
    text(t(`compare.m.${k}.gloss` as 'compare.m.contrast'), PAD, y + 40, { font: '11px Inter, sans-serif', color: c.mute });
    data.faces.forEach((f, i) => {
      const m = f.metrics[k]; const bx = x0(i);
      if (m.value === null) { text(t('compare.na'), bx, y + 28, { color: c.mute }); return; }
      text(`${numberText(m.value, m.unit)}${m.quality === 'estimated' ? ' · ' + t('compare.est') : ''}`, bx, y + 24, { font: '600 15px Inter, sans-serif' });
      const d = i === 0 ? null : deltaOf(k, m, base.metrics[k]);
      if (d) text(deltaText(d).text, bx + 130, y + 24, { font: '600 11px Inter, sans-serif', color: c.accent });
      if (m.percentile !== null) {
        ctx.fillStyle = c.line; ctx.fillRect(bx, y + 36, CW - 40, 4);
        ctx.fillStyle = c.accent; ctx.fillRect(bx + (CW - 40) * Math.min(1, Math.max(0, m.percentile / 100)) - 1, y + 32, 2, 12);
        text(percentLabel(m.percentile), bx + CW - 36, y + 42, { font: '10px Inter, sans-serif', color: c.mute });
      }
    });
    y += ROW;
  }
  ctx.strokeStyle = c.line; ctx.beginPath(); ctx.moveTo(PAD, y); ctx.lineTo(W - PAD, y); ctx.stroke();
  text(t('compare.row.class'), PAD, y + 28, { font: '600 12px Inter, sans-serif' });
  data.faces.forEach((f, i) => {
    const k = f.classification;
    text(`${classLabel(k.class)}${k.subclass ? ' · ' + subclassLabel(k.subclass) : ''}${k.traits?.length ? ' · ' + k.traits.join(', ') : ''}`, x0(i), y + 28, { font: '600 13px Inter, sans-serif' });
    text(`${Math.round(k.confidence * 100)}%`, x0(i), y + 46, { font: '11px Inter, sans-serif', color: c.mute });
    text(`${t('compare.row.license')}: ${f.license ?? '—'} · ${t('compare.licenseNote')}`, x0(i), y + 66, { font: '11px Inter, sans-serif', color: c.mute });
  });
  return canvas;
}

export async function exportComparePng(input: ExportInput): Promise<string> {
  const canvas = renderCompareCanvas(input);
  if (!canvas) throw new Error('canvas 2D không khả dụng');
  const blob: Blob | null = await new Promise(res => canvas.toBlob(res, 'image/png'));
  if (!blob) throw new Error('không tạo được ảnh');
  const name = exportFileName(input.data);
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a'); a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
  return name;
}
