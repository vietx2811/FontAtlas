import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { COMPARE_METRICS, type CompareDto } from '../../api/compare';
import { hasKey, t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import { ClassCell, MetricCell, OverlayCell, SpecimenCell, ViCell, WithFamily } from './CompareCells';
import { fitScale } from './compareMath';
import { exportComparePng } from './exportPng';
import './compare.css';

const SPEC_SIZE = 30;
const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const label = (k: string, gloss = false): string => { const key = `compare.m.${k}${gloss ? '.gloss' : ''}`; return hasKey(key) ? t(key) : gloss ? '' : k; };

/** Màn Compare: 2–8 cột, cột đầu là mốc; toàn bộ số đo là số thật từ phân tích (không có thì ghi "Không đo được"). */
export function ComparePage({ api, faces }: { api: Api; faces: FaceKeyDto[] }): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const [data, setData] = useState<CompareDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [fit, setFit] = useState(false);
  const [overlay, setOverlay] = useState(true);
  const families = useRef(new Map<string, string | null>());
  const close = () => useUiStore.getState().closeCompare();
  const idsKey = faces.map(keyOf).join('|');

  useEffect(() => {
    let alive = true; setData(null); setError(null);
    api.compareFaces(faces).then(d => { if (alive) setData(d); }, e => { if (alive) setError(e instanceof Error ? e.message : String(e)); });
    return () => { alive = false; };
  }, [api, idsKey, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented || document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]')) return;
      close();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);

  const exportPng = async () => {
    if (!data) return;
    try {
      const name = await exportComparePng({ data, families: families.current, sample, fit, dark: document.documentElement.dataset.theme === 'dark' || (!document.documentElement.dataset.theme && matchMedia('(prefers-color-scheme: dark)').matches) });
      useNoticeStore.getState().push({ kind: 'info', text: t('compare.pngDone', { name }) });
    } catch (e) {
      useNoticeStore.getState().push({ kind: 'error', text: t('compare.pngFail', { msg: e instanceof Error ? e.message : String(e) }) });
    }
  };

  const header = (
    <div className="cmp-h">
      <Tooltip label={t('compare.back')} shortcut="Esc"><button type="button" className="cmp-back" aria-label={t('compare.back')} onClick={close}><Icon name="x" size={14} /></button></Tooltip>
      <h1>{t('compare.title')} · {faces.length}</h1>
      <div className="cmp-tog" role="group" aria-label={`${t('compare.sameSize')} / ${t('compare.matchXh')}`}>
        <Tooltip label={`${t('compare.sameSize')} — ${t('compare.sameSize.gloss')}`}><button type="button" aria-pressed={!fit} onClick={() => setFit(false)}>{t('compare.sameSize')}</button></Tooltip>
        <Tooltip label={`${t('compare.matchXh')} — ${t('compare.matchXh.gloss')}`}><button type="button" aria-pressed={fit} onClick={() => setFit(true)}>{t('compare.matchXh')}</button></Tooltip>
      </div>
      <div className="cmp-tog"><Tooltip label={`${t('compare.overlay')} — ${t('compare.overlay.gloss')}`}><button type="button" aria-pressed={overlay} onClick={() => setOverlay(v => !v)}>{t('compare.overlay')}</button></Tooltip></div>
      <span className="cmp-sp" />
      <button type="button" className="cmp-btn" disabled={!data} onClick={() => void exportPng()}><Icon name="plus" size={13} />{t('compare.png')}</button>
      <Tooltip label={t('compare.pdfSoon')}><button type="button" className="cmp-btn" aria-disabled="true" onClick={e => e.preventDefault()}>{t('compare.pdf')}</button></Tooltip>
    </div>
  );

  if (error) return <div className="cmp">{header}<div role="alert" className="cmp-state">{t('compare.error', { msg: error })} <button type="button" onClick={() => setAttempt(n => n + 1)}>{t('compare.retry')}</button></div></div>;
  if (!data) return <div className="cmp">{header}<div role="status" className="cmp-state">{t('compare.loading')}</div></div>;

  const base = data.faces[0];
  const scale = (f: typeof base) => (fit && f !== base ? fitScale(base.metrics.x_height_em?.value ?? null, f.metrics.x_height_em?.value ?? null) : 1);
  const cells = <T,>(fn: (f: typeof base) => T) => data.faces.map(f => fn(f));
  return (
    <div className="cmp">
      {header}
      <div className="cmp-body">
        <div className="cmp-tbl" role="table" aria-label={t('compare.title')} style={{ ['--n' as string]: data.faces.length }}>
          <div className="rh" role="columnheader" />
          {cells(f => <div key={keyOf(f.face)} role="columnheader" className={`ch${f === base ? ' base' : ''}`}>
            <WithFamily api={api} face={f}>{fam => { families.current.set(keyOf(f.face), fam); return null; }}</WithFamily>
            <div className="t">{f === base && <Icon name="anchor" size={13} />}{f.row.family}</div>
            <div className="m">{f === base ? t('compare.base') : f.row.subfamily}{f === base && f.row.subfamily ? ` · ${f.row.subfamily}` : ''}</div>
          </div>)}

          <div className="gh">{t('compare.sec.specimen')}</div>
          <div className="rh"><b>{t('compare.row.line')}</b></div>
          {cells(f => <div key={keyOf(f.face)} role="cell"><SpecimenCell api={api} face={f} text={sample || 'Hxg'} size={SPEC_SIZE * scale(f)} /></div>)}
          {overlay && <><div className="rh"><b>{t('compare.row.overlay')}</b><small>H x g ă</small></div>
            {cells(f => <div key={keyOf(f.face)} role="cell"><OverlayCell api={api} base={base} face={f} scale={scale(f)} /></div>)}</>}
          <div className="rh"><b>{t('compare.row.vi')}</b><small>Ữ Ặ Ệ Ỗ Ở · ươ</small></div>
          {cells(f => <div key={keyOf(f.face)} role="cell"><ViCell api={api} face={f} size={Math.round(SPEC_SIZE * 1.1 * scale(f))} /></div>)}

          <div className="gh">{t('compare.sec.metrics')}</div>
          {COMPARE_METRICS.map(k => <div key={k} className="row" role="row">
            <div className="rh" role="rowheader"><b>{label(k)}</b><small>{label(k, true)}</small></div>
            {cells(f => <div key={keyOf(f.face)} role="cell"><MetricCell k={k} m={f.metrics[k]} base={base.metrics[k]} isBase={f === base} /></div>)}
          </div>)}

          <div className="gh">{t('compare.sec.class')}</div>
          <div className="rh"><b>{t('compare.row.class')}</b></div>
          {cells(f => <div key={keyOf(f.face)} role="cell"><ClassCell face={f} /></div>)}

          <div className="gh">{t('compare.sec.meta')}</div>
          <div className="rh"><b>{t('compare.row.license')}</b></div>
          {cells(f => <div key={keyOf(f.face)} role="cell" className="cmp-lic"><b>{f.license ?? '—'}</b><span>{t('compare.licenseNote')}</span></div>)}
        </div>
        <p className="cmp-foot">{t('compare.footnote')}</p>
      </div>
    </div>
  );
}
