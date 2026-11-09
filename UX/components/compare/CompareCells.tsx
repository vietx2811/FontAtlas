import type { Api } from '../../api';
import type { CompareFace, CompareMetric } from '../../api/compare';
import { hasKey, t } from '../../i18n';
import { numberText, unitText } from '../detail/tabs/measurements';
import { useFontFamily } from '../detail/useFontFamily';
import { classLabel, subclassLabel, traitLabel } from '../main/classLabels';
import { TermTip, classTermKey, subclassTermKey } from '../ui/terms';
import { useInk } from '../ui/inkLines';
import { NoteMark } from '../ui/noteMark';
import { deltaOf, deltaText, percentLabel, xhOk } from './compareMath';

export const SAMPLE_VI = 'ỮẶỆỖỞ ươ';
export const SAMPLE_VI_GLOSS = 'Ữ Ặ Ệ Ỗ Ở · ươ';
export const OVERLAY_TEXT = 'Hxgă';
export const OVERLAY_SPACED = [...OVERLAY_TEXT].join(' ');

/** Nạp font của một cột rồi đưa họ font cho phần vẽ (null khi chưa nạp: dùng font UI dự phòng). */
export function WithFamily({ api, face, children }: { api: Api; face: CompareFace; children(family: string | null): React.ReactNode }): React.JSX.Element {
  const family = useFontFamily(api, { hashHex: face.face.hashHex, faceIndex: face.face.faceIndex, fileSize: face.row.fileSize });
  return <>{children(family)}</>;
}
const css = (family: string | null) => (family ? `"${family}", var(--font-ui)` : 'var(--font-ui)');
const tr = (key: string, fallback: string): string => (hasKey(key) ? t(key) : fallback);

/** Cột thiếu x-height khi bật Match x-height: không đổi cỡ, nói rõ vì sao. */
export const noXh = (face: CompareFace, fit: boolean) => fit && !xhOk(face.metrics.x_height_em?.value);

export function SpecimenCell({ api, face, text, size, fit }: { api: Api; face: CompareFace; text: string; size: number; fit: boolean }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => (
    <div className="cmp-page fa-note-host">
      <div className="cmp-line" style={{ fontFamily: css(fam), fontSize: size }}>{text}</div>
      {noXh(face, fit) && <NoteMark text={t('compare.noXh')} />}
    </div>
  )}</WithFamily>;
}

const PLAIN = { fontWeight: 'normal', fontStyle: 'normal', fontSynthesis: 'none', fontVariationSettings: 'normal' } as const;
/** Glyph của mốc (xám) chồng lên glyph của cột (màu), chung baseline. Đường đứt = x-height ĐO TỪ CHỮ MỐC đang vẽ (canvas ink), không theo số analyzer; mốc chưa nạp thì không vẽ đường. */
export function OverlayCell({ api, base, face, scale }: { api: Api; base: CompareFace; face: CompareFace; scale: number }): React.JSX.Element {
  return (
    <WithFamily api={api} face={base}>{bf => <WithFamily api={api} face={face}>{ff => <OverlayArt base={base} face={face} scale={scale} bf={bf} ff={ff} />}</WithFamily>}</WithFamily>
  );
}
function OverlayArt({ base, face, scale, bf, ff }: { base: CompareFace; face: CompareFace; scale: number; bf: string | null; ff: string | null }): React.JSX.Element {
  const W = 300, H = 96, Y = 66, SZ = 64;
  const ink = useInk(bf);
  const baseXh = ink?.x ?? null;
  const same = face === base;
  return (
    <div className="cmp-page cmp-overlay">
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMinYMid meet" role="img" aria-label={`${t('compare.row.overlay')} ${face.row.family}`} data-xh={baseXh === null ? '' : String(baseXh)}>
        <line x1="0" x2={W} y1={Y} y2={Y} className="bl" />
        {baseXh !== null && <line data-testid="overlay-xh" x1="0" x2={W} y1={Y - baseXh * SZ} y2={Y - baseXh * SZ} className="xh" />}
        {!same && <text x="8" y={Y} fontSize={SZ} fontFamily={css(bf)} style={PLAIN} className="ghost">{OVERLAY_TEXT}</text>}
        <text x="8" y={Y} fontSize={SZ * scale} fontFamily={css(ff)} style={PLAIN} className={same ? 'solo' : 'over'}>{OVERLAY_TEXT}</text>
      </svg>
    </div>
  );
}

const pct1 = (v: number) => new Intl.NumberFormat('vi-VN', { maximumFractionDigits: 1, useGrouping: false }).format(v);
/** Chữ cảnh báo tràn: % em vượt hhea.ascender (backend); thiếu thì lùi về đỉnh dấu chồng tính bằng em. */
function overText(face: CompareFace): string {
  const { overAscenderPctEm: a, overCapPctEm: c, stackedMarksTopEm: em } = face.vi;
  const p = a != null && a > 0 ? a : c;
  if (p != null && p > 0) return t('compare.viWarnPct', { pct: pct1(p) });
  return em !== null ? t('compare.viWarnEm', { em: numberText(em, 'em') }) : t('compare.viWarn');
}

export type ViNote = { kind: 'warn' | 'ok'; text: string; title?: string };
/** Nhãn dưới dòng dấu chồng (dùng chung cho màn hình và ảnh PNG): thiếu ký tự / không hỗ trợ / tràn khung / vừa khung. */
export function viNotes(face: CompareFace): ViNote[] {
  const { stackedMarksOverflow: over, supports, missing } = face.vi;
  const out: ViNote[] = [];
  if (!supports) out.push({ kind: 'warn', text: missing > 0 ? t('compare.viMissing', { n: missing }) : t('compare.viNone') });
  if (supports && over) out.push({ kind: 'warn', text: overText(face) });
  if (supports && !over) out.push({ kind: 'ok', text: t('compare.viOk'), title: face.vi.overCapPctEm != null ? t('compare.viOkTip', { pct: pct1(face.vi.overCapPctEm) }) : undefined });
  if (!supports && over) out.push({ kind: 'warn', text: t('compare.viWarn') });
  return out;
}

export function ViCell({ api, face, size }: { api: Api; face: CompareFace; size: number }): React.JSX.Element {
  const { stackedMarksOverflow: over, supports } = face.vi;
  const notes = viNotes(face);
  return (
    <WithFamily api={api} face={face}>{fam => (
      <div className="cmp-page fa-note-host">
        <div className="cmp-vi" data-overflow={over} data-unsupported={!supports} style={{ fontFamily: css(fam), fontSize: size }}>{SAMPLE_VI}</div>
        <NoteMark testId="vi-note" kind={notes.some(n => n.kind === 'warn') ? 'warn' : 'info'} text={notes.map(n => n.title ? `${n.text} — ${n.title}` : n.text).join(' · ')} />
      </div>
    )}</WithFamily>
  );
}

export function MetricCell({ k, m, base, isBase }: { k: string; m: CompareMetric; base: CompareMetric; isBase: boolean }): React.JSX.Element {
  if (m.value === null) return <div className="cmp-na"><span>{t('compare.na')}</span>{m.reason && <NoteMark inline kind="info" text={m.reason} />}</div>;
  const d = isBase ? null : deltaOf(k, m, base);
  const dt = d ? deltaText(d) : null;
  const baseMissing = !isBase && base.value === null;
  const p = m.percentile;
  return (
    <div className="cmp-metric">
      <div className="cmp-val"><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>
        {m.quality === 'estimated' && <NoteMark inline kind="info" text={t('compare.est')} />}
        {isBase ? <span className="cmp-dl zero">{t('compare.base')}</span> : dt ? <span className={`cmp-dl ${dt.tone}`}>{dt.text}</span> : <span className="cmp-dl zero" title={baseMissing ? t('compare.baseNa') : t('compare.noDelta')}>—</span>}
      </div>
      {p !== null && <div className="cmp-pbar" role="img" aria-label={t('compare.percentile', { p: Math.round(Math.min(100, Math.max(0, p))) })}><i style={{ left: `${Math.min(100, Math.max(0, p))}%` }} /><em style={{ left: `${Math.min(92, Math.max(8, p))}%` }}>{percentLabel(p)}</em></div>}
    </div>
  );
}

export function ClassCell({ face }: { face: CompareFace }): React.JSX.Element {
  const c = face.classification;
  const traits = c.traits ?? [];
  const usage = c.usage ?? [];
  const uncertain = c.class === 'uncertain';
  const conf = Number.isFinite(c.confidence) ? Math.round(c.confidence * 100) : null;
  return (
    <div className="cmp-cls">
      <span className="c" data-uncertain={uncertain}><TermTip term={classTermKey(c.class)}><span data-term={classTermKey(c.class)}>{classLabel(c.class)}</span></TermTip>{c.source === 'override' && <NoteMark inline kind="info" text={t('class.edited')} />}</span>
      {uncertain && c.candidate && <span className="s">{t('class.maybe', { label: classLabel(c.candidate) })}</span>}
      {!uncertain && c.subclass && <TermTip term={subclassTermKey(c.subclass)}><span className="s" data-term={subclassTermKey(c.subclass)}>{subclassLabel(c.subclass)}</span></TermTip>}
      {traits.length > 0 && <span className="s">{traits.map(traitLabel).join(' · ')}</span>}
      {usage.length > 0 && <span className="s">{usage.map(u => tr(`class.usage.${u}`, u)).join(' · ')}</span>}
      {conf !== null && <div className="cmp-conf" role="img" aria-label={`${conf}%`}><i style={{ width: `${conf}%` }} /></div>}
    </div>
  );
}
