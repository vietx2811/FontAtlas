import { useRef } from 'react';
import { Tooltip } from '../ui/Tooltip';
import type { Api } from '../../api';
import { useUiStore } from '../../store/ui';
import type { CompareFace, CompareMetric } from '../../api/compare';
import { hasKey, t } from '../../i18n';
import { numberText, unitText } from '../detail/tabs/measurements';
import { useFontFamily } from '../detail/useFontFamily';
import { classLabel, subclassLabel, traitLabel } from '../main/classLabels';
import { TermTip, classTermKey, subclassTermKey } from '../ui/terms';
import { useInk } from '../ui/inkLines';
import { NoteMark } from '../ui/noteMark';
import { graphemes, missingList, useMissingChars } from './glyphCoverage';
import { barFill, deltaOf, deltaText, xhOk, type BarScale } from './compareMath';

export const SAMPLE_VI = 'ỮẶỆỖỞ ươ';
export const PLACEHOLDER = 'Hxg';
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

/** Chữ mẫu sửa tại chỗ: một ô nhập trong suốt nằm trên lớp hiển thị (kiểu chữ giữ nguyên); sửa ở một cột là đổi ở mọi cột (`useUiStore.sample`). Ký tự font không có hiện xám (`--faint`). */
export function SpecimenCell({ api, face, text, size, fit }: { api: Api; face: CompareFace; text: string; size: number; fit: boolean }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => <SpecimenArt face={face} fam={fam} text={text} size={size} fit={fit} />}</WithFamily>;
}
function SpecimenArt({ face, fam, text, size, fit }: { face: CompareFace; fam: string | null; text: string; size: number; fit: boolean }): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const missing = useMissingChars(fam, text);
  const layer = useRef<HTMLDivElement>(null);
  const sync = (el: HTMLInputElement) => { if (layer.current) layer.current.style.transform = `translateX(${-el.scrollLeft}px)`; };
  const style = { fontFamily: css(fam), fontSize: size } as const;
  return (
    <div className="cmp-page fa-note-host cmp-spec">
      <div className="cmp-line cmp-spec-view" aria-hidden="true" style={style}><div ref={layer}>{text === '' ? <span className="ph">{PLACEHOLDER}</span> : graphemes(text).map((ch, i) => <span key={i} className={missing.has(ch) ? 'miss' : undefined}>{ch}</span>)}</div></div>
      <input className="cmp-edit" type="text" value={sample} placeholder={PLACEHOLDER} spellCheck={false} autoComplete="off" style={style} aria-label={`${t('compare.edit')} · ${face.row.family}`}
        onChange={e => { useUiStore.getState().setSample(e.target.value); sync(e.target); }} onKeyDown={e => { if (e.key === 'Escape' && !e.nativeEvent.isComposing && e.keyCode !== 229) { e.stopPropagation(); e.currentTarget.blur(); } }} onScroll={e => sync(e.currentTarget)} onKeyUp={e => sync(e.currentTarget)} onSelect={e => sync(e.currentTarget)} />
      {missing.size > 0 && <NoteMark testId="missing-note" text={t('compare.missing', { chars: missingList(missing) })} />}
      {noXh(face, fit) && <NoteMark text={t('compare.noXh')} />}
    </div>
  );
}

const PLAIN = { fontWeight: 'normal', fontStyle: 'normal', fontSynthesis: 'none', fontVariationSettings: 'normal' } as const;
/** Kiểu vẽ các lớp overlay: mốc luôn đặc; các font khác là nét liền / nét đứt / chấm, tông đen → xám. */
export const OVERLAY_STYLES = ['line', 'dash', 'dot'] as const;
export type OverlayStyle = 'solid' | (typeof OVERLAY_STYLES)[number];
export const overlayStyleOf = (selected: readonly CompareFace[], base: CompareFace): Map<CompareFace, OverlayStyle> => {
  const m = new Map<CompareFace, OverlayStyle>(); let k = 0;
  for (const f of selected) m.set(f, f === base ? 'solid' : OVERLAY_STYLES[Math.min(k++, OVERLAY_STYLES.length - 1)]);
  return m;
};
export const faceName = (f: CompareFace): string => `${f.row.family} ${f.row.subfamily ?? ''}`.trim();
const OV = { W: 520, H: 108, Y: 80, SZ: 72 } as const;

/** Vùng Overlay chung: các font được chọn (≤ 4) chồng lên nhau chung baseline. Mốc vẽ đặc; font khác vẽ nét. Đường đứt = x-height ĐO TỪ CHỮ đang vẽ của font chính (mốc, nếu mốc được chọn). */
export function OverlayPanel({ api, base, selected, scale }: { api: Api; base: CompareFace; selected: readonly CompareFace[]; scale(f: CompareFace): number }): React.JSX.Element {
  const styles = overlayStyleOf(selected, base);
  const primary = selected.includes(base) ? base : selected[0];
  return (
    <section className="cmp-ovl" aria-label={t('compare.row.overlay')} data-count={selected.length}>
      <div className="cmp-ovl-h"><TermTip term="overlay"><b data-term="overlay">{t('compare.row.overlay')}</b></TermTip><small>{OVERLAY_SPACED}</small></div>
      {selected.length === 0 ? <p className="cmp-ovl-empty">{t('compare.ov.empty')}</p> : (
        <svg viewBox={`0 0 ${OV.W} ${OV.H}`} preserveAspectRatio="xMinYMid meet" role="img" aria-label={`${t('compare.row.overlay')}: ${selected.map(faceName).join(', ')}`}>
          <line x1="0" x2={OV.W} y1={OV.Y} y2={OV.Y} className="bl" />
          {selected.map(f => <OverlayLayer key={`${f.face.hashHex}:${f.face.faceIndex}`} api={api} face={f} style={styles.get(f)!} scale={scale(f)} primary={f === primary} />)}
        </svg>
      )}
      <ul className="cmp-ovl-legend">
        {selected.map(f => (
          <li key={`${f.face.hashHex}:${f.face.faceIndex}`} data-style={styles.get(f)}>
            <svg width="26" height="10" aria-hidden="true"><line x1="1" x2="25" y1="5" y2="5" className={`ov-${styles.get(f)} sw`} /></svg>
            <span>{faceName(f)}{f === base ? ` · ${t('compare.base')}` : ''}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
function OverlayLayer({ api, face, style, scale, primary }: { api: Api; face: CompareFace; style: OverlayStyle; scale: number; primary: boolean }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => <OverlayLayerArt fam={fam} style={style} scale={scale} primary={primary} />}</WithFamily>;
}
function OverlayLayerArt({ fam, style, scale, primary }: { fam: string | null; style: OverlayStyle; scale: number; primary: boolean }): React.JSX.Element {
  const ink = useInk(primary ? fam : null);
  const xh = primary ? ink?.x ?? null : null;
  return (
    <>
      {xh !== null && <line data-testid="overlay-xh" x1="0" x2={OV.W} y1={OV.Y - xh * OV.SZ} y2={OV.Y - xh * OV.SZ} className="xh" />}
      <text x="8" y={OV.Y} fontSize={OV.SZ * scale} fontFamily={css(fam)} style={PLAIN} className={`ov-${style}`} data-style={style}>{OVERLAY_TEXT}</text>
    </>
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

/** Chỉ số (theo ký tự) của các ký tự mang dấu chồng: sau khi tách NFD có từ 2 dấu kết hợp trở lên (Ữ, Ặ, Ệ, ấ, ở…). */
export function stackedIndexes(text: string): Set<number> {
  const out = new Set<number>();
  graphemes(text).forEach((ch, i) => { if ([...ch.normalize('NFD')].filter(c => /\p{Mn}/u.test(c)).length >= 2) out.add(i); });
  return out;
}

export function ViCell({ api, face, size }: { api: Api; face: CompareFace; size: number }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => <ViArt face={face} fam={fam} size={size} />}</WithFamily>;
}
function ViArt({ face, fam, size }: { face: CompareFace; fam: string | null; size: number }): React.JSX.Element {
  const { stackedMarksOverflow: over, supports } = face.vi;
  const notes = viNotes(face);
  const missing = useMissingChars(fam, SAMPLE_VI);
  const stacked = stackedIndexes(SAMPLE_VI);
  const parts = [...notes.map(n => (n.title ? `${n.text} — ${n.title}` : n.text)), ...(missing.size ? [t('compare.missing', { chars: missingList(missing) })] : [])];
  return (
    <div className="cmp-page fa-note-host">
      <div className="cmp-vi" data-overflow={over} data-unsupported={!supports} style={{ fontFamily: css(fam), fontSize: size }}>
        {graphemes(SAMPLE_VI).map((ch, i) => <span key={i} className={`${stacked.has(i) ? 'stk' : ''}${missing.has(ch) ? ' miss' : ''}`.trim() || undefined} data-stacked={stacked.has(i) ? 'true' : undefined}>{ch}</span>)}
      </div>
      <NoteMark testId="vi-note" kind={notes.some(n => n.kind === 'warn') || missing.size > 0 ? 'warn' : 'info'} text={parts.join(' · ')} />
    </div>
  );
}

export function MetricCell({ k, m, base, isBase, scale }: { k: string; m: CompareMetric; base: CompareMetric; isBase: boolean; scale: BarScale | null }): React.JSX.Element {
  if (m.value === null) return <div className="cmp-na"><span>{t('compare.na')}</span>{m.reason && <NoteMark inline kind="info" text={m.reason} />}</div>;
  const d = isBase ? null : deltaOf(k, m, base);
  const dt = d ? deltaText(d) : null;
  const baseMissing = !isBase && base.value === null;
  const p = m.percentile;
  const pl = p === null ? null : t('compare.percentile', { p: Math.round(Math.min(100, Math.max(0, p))) });
  return (
    <div className="cmp-metric">
      <div className="cmp-val"><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>
        {m.quality === 'estimated' && <NoteMark inline kind="info" text={t('compare.est')} />}
        {isBase ? <span className="cmp-dl zero">{t('compare.base')}</span> : dt ? <span className={`cmp-dl ${dt.tone}`}>{dt.text}</span> : <span className="cmp-dl zero" title={baseMissing ? t('compare.baseNa') : t('compare.noDelta')}>—</span>}
      </div>
      {scale && (() => { const f = barFill(m.value, scale); return (
        <Tooltip label={[t('compare.pbar.tip'), pl].filter(Boolean).join(' · ')}>
          <div className="cmp-pbar" role="img" aria-label={pl ?? t('compare.pbar.aria')} data-signed={scale.signed || undefined}><i className="fill" style={{ left: `${f.left}%`, width: `${f.width}%` }} /></div>
        </Tooltip>
      ); })()}
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
