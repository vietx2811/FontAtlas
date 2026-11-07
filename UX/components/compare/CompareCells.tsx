import type { Api } from '../../api';
import type { CompareFace, CompareMetric } from '../../api/compare';
import { hasKey, t } from '../../i18n';
import { numberText, unitText } from '../detail/tabs/measurements';
import { useFontFamily } from '../detail/useFontFamily';
import { classLabel, subclassLabel } from '../main/classLabels';
import { deltaOf, deltaText, percentLabel } from './compareMath';

export const SAMPLE_VI = 'ỮẶỆỖỞ ươ';
export const OVERLAY_TEXT = 'Hxgă';

/** Nạp font của một cột rồi đưa họ font cho phần vẽ (null khi chưa nạp: dùng font UI dự phòng). */
export function WithFamily({ api, face, children }: { api: Api; face: CompareFace; children(family: string | null): React.ReactNode }): React.JSX.Element {
  const family = useFontFamily(api, { hashHex: face.face.hashHex, faceIndex: face.face.faceIndex, fileSize: face.row.fileSize });
  return <>{children(family)}</>;
}
const css = (family: string | null) => (family ? `"${family}", var(--font-ui)` : 'var(--font-ui)');
const tr = (key: string, fallback: string): string => (hasKey(key) ? t(key) : fallback);

export function SpecimenCell({ api, face, text, size }: { api: Api; face: CompareFace; text: string; size: number }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => <div className="cmp-page"><div className="cmp-line" style={{ fontFamily: css(fam), fontSize: size }}>{text}</div></div>}</WithFamily>;
}

/** Glyph của mốc (xám) chồng lên glyph của cột (màu nhấn), chung baseline; đường đứt = x-height của mốc. */
export function OverlayCell({ api, base, face, scale }: { api: Api; base: CompareFace; face: CompareFace; scale: number }): React.JSX.Element {
  const W = 300, H = 96, Y = 66, SZ = 64;
  const baseXh = base.metrics.x_height_em?.value ?? 0.5;
  const same = face === base;
  return (
    <WithFamily api={api} face={base}>{bf => <WithFamily api={api} face={face}>{ff => (
      <div className="cmp-page cmp-overlay">
        <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMinYMid meet" role="img" aria-label={`${t('compare.row.overlay')} ${face.row.family}`}>
          <line x1="0" x2={W} y1={Y} y2={Y} className="bl" />
          <line x1="0" x2={W} y1={Y - baseXh * SZ} y2={Y - baseXh * SZ} className="xh" />
          {!same && <text x="8" y={Y} fontSize={SZ} fontFamily={css(bf)} className="ghost">{OVERLAY_TEXT}</text>}
          <text x="8" y={Y} fontSize={SZ * scale} fontFamily={css(ff)} className={same ? 'solo' : 'over'}>{OVERLAY_TEXT}</text>
        </svg>
      </div>
    )}</WithFamily>}</WithFamily>
  );
}

export function ViCell({ api, face, size }: { api: Api; face: CompareFace; size: number }): React.JSX.Element {
  const { stackedMarksOverflow: over, stackedMarksTopEm: em, supports, missing } = face.vi;
  return (
    <WithFamily api={api} face={face}>{fam => (
      <div className="cmp-page">
        <div className="cmp-vi" data-overflow={over} style={{ fontFamily: css(fam), fontSize: size }}>{SAMPLE_VI}</div>
        {!supports && missing > 0 && <span className="cmp-chip warn">{t('compare.viMissing', { n: missing })}</span>}
        {over
          ? <span className="cmp-chip warn" role="status">{em !== null ? t('compare.viWarnEm', { em: numberText(em, 'em') }) : t('compare.viWarn')}</span>
          : <span className="cmp-ok">{t('compare.viOk')}</span>}
      </div>
    )}</WithFamily>
  );
}

export function MetricCell({ k, m, base, isBase }: { k: string; m: CompareMetric; base: CompareMetric; isBase: boolean }): React.JSX.Element {
  if (m.value === null) return <div className="cmp-na" title={m.reason ?? undefined}><span>{t('compare.na')}</span>{m.reason && <small>{m.reason}</small>}</div>;
  const d = isBase ? null : deltaOf(k, m, base);
  const dt = d ? deltaText(d) : null;
  const p = m.percentile;
  return (
    <div className="cmp-metric">
      <div className="cmp-val"><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>
        {m.quality === 'estimated' && <span className="cmp-est">{t('compare.est')}</span>}
        {isBase ? <span className="cmp-dl zero">{t('compare.base')}</span> : dt && <span className={`cmp-dl ${dt.tone}`}>{dt.text}</span>}
      </div>
      {p !== null && <div className="cmp-pbar" role="img" aria-label={percentLabel(p)}><i style={{ left: `${Math.min(100, Math.max(0, p))}%` }} /><em style={{ left: `${Math.min(92, Math.max(8, p))}%` }}>{percentLabel(p)}</em></div>}
    </div>
  );
}

export function ClassCell({ face }: { face: CompareFace }): React.JSX.Element {
  const c = face.classification;
  const traits = c.traits ?? [];
  const uncertain = c.class === 'uncertain';
  return (
    <div className="cmp-cls">
      <span className="c" data-uncertain={uncertain}>{classLabel(c.class)}{c.source === 'override' && <span className="cmp-est">{t('class.edited')}</span>}</span>
      {uncertain && c.candidate && <span className="s">{t('compare.maybe', { label: classLabel(c.candidate) })}</span>}
      {!uncertain && c.subclass && <span className="s">{subclassLabel(c.subclass)}</span>}
      {traits.length > 0 && <span className="s">{traits.join(' · ')}</span>}
      {c.usage.length > 0 && <span className="s">{c.usage.map(u => tr(`class.usage.${u}`, u)).join(' · ')}</span>}
      <div className="cmp-conf" role="img" aria-label={`${Math.round(c.confidence * 100)}%`}><i style={{ width: `${Math.round(c.confidence * 100)}%` }} /></div>
    </div>
  );
}
