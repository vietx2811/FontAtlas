import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { COMPARE_METRICS, type CompareDto, type CompareFace } from '../../api/compare';
import { hasKey, t, type I18nKey } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import { TermTip, termKey, termName } from '../ui/terms';
import { ClassCell, MetricCell, OVERLAY_SPACED, OverlayCell, SpecimenCell, ViCell, WithFamily } from './CompareCells';
import { META_ROWS, LicenseCell } from './MetaCells';
import { fitScale } from './compareMath';
import { ensureFonts, renderCompare, renderCompareBlob } from './exportPng';
import './compare.css';

const SPEC_SIZE = 30;
const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const label = (k: string, gloss = false): string => { if (!gloss) return termName(k); const key = `compare.m.${k}.gloss`; return hasKey(key) ? t(key) : ''; };
const gl = (key: string): string => (hasKey(key) ? t(key as I18nKey) : '');

/** Ghi họ font đã nạp của một cột (cho xuất PNG) sau khi vẽ, không ghi trong lúc render. */
function FamilyTap({ api, face, into }: { api: Api; face: CompareFace; into: Map<string, string | null> }): React.JSX.Element {
  return <WithFamily api={api} face={face}>{fam => <Record id={keyOf(face.face)} fam={fam} into={into} />}</WithFamily>;
}
function Record({ id, fam, into }: { id: string; fam: string | null; into: Map<string, string | null> }): null {
  useEffect(() => { into.set(id, fam); }, [id, fam, into]);
  return null;
}
const Row = ({ children }: { children: React.ReactNode }) => <div className="row" role="row">{children}</div>;
const Group = ({ k, cols }: { k: 'specimen' | 'metrics' | 'class' | 'meta'; cols: number }) => (
  <Row><div role="columnheader" aria-colspan={cols} className="gh">{t(`compare.sec.${k}` as I18nKey)}{gl(`compare.sec.${k}.gloss`) && <small> · {gl(`compare.sec.${k}.gloss`)}</small>}</div></Row>
);

/** Màn Compare: 2–8 cột, cột đầu là mốc; toàn bộ số đo là số thật từ phân tích (không có thì ghi "Không đo được"). */
export function ComparePage({ api, faces }: { api: Api; faces: FaceKeyDto[] }): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const [data, setData] = useState<CompareDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [fit, setFit] = useState(false);
  const [overlay, setOverlay] = useState(true);
  const [exporting, setExporting] = useState(false);
  const families = useRef(new Map<string, string | null>());
  const title = useRef<HTMLHeadingElement>(null);
  const close = () => useUiStore.getState().closeCompare();
  const idsKey = faces.map(keyOf).join('|');

  useEffect(() => {
    let alive = true; setData(null); setError(null);
    api.compareFaces(faces).then(d => { if (alive) setData(d); }, e => { if (alive) setError(e instanceof Error ? e.message : String(e)); });
    return () => { alive = false; };
  }, [api, idsKey, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  // Tiêu điểm vào màn khi mở, trả về chỗ cũ khi đóng.
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    title.current?.focus();
    return () => { if (before && before.isConnected) before.focus(); };
  }, []);

  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented || document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]')) return;
      close();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);

  const isDark = () => document.documentElement.dataset.theme === 'dark' || (!document.documentElement.dataset.theme && typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches);

  // Móc QA native (chỉ khi chạy kịch bản QA): vẽ PNG như nút Xuất, không mở hộp thoại, trả bản kê nội dung + mẫu điểm ảnh.
  useEffect(() => {
    if (!data || !(window as unknown as { __FA_QA_PICKS__?: unknown }).__FA_QA_PICKS__) return;
    (window as unknown as { __faQaComparePng?: () => Promise<unknown> }).__faQaComparePng = async () => {
      await ensureFonts(families.current.values(), sample);
      const r = renderCompare({ data, families: families.current, sample, fit, dark: isDark(), overlay });
      if (!r) return null;
      const px = r.canvas.getContext('2d')!.getImageData(2, r.canvas.height - 2, 1, 1).data;
      return { ...r.manifest, cornerPx: [px[0], px[1], px[2]], dataUrl: r.canvas.toDataURL('image/png') };
    };
    return () => { delete (window as unknown as { __faQaComparePng?: unknown }).__faQaComparePng; };
  }, [data, sample, fit, overlay]);

  const exportPng = async () => {
    if (!data || exporting) return;
    setExporting(true);
    try {
      const { blob, name } = await renderCompareBlob({ data, families: families.current, sample, fit, dark: isDark(), overlay });
      const saved = await api.savePng(blob, name);
      if (saved) useNoticeStore.getState().push({ kind: 'info', text: t('compare.pngSaved', { name: saved.split(/[\\/]/).pop() ?? saved }) });
    } catch (e) {
      useNoticeStore.getState().push({ kind: 'error', text: t('compare.pngFail', { msg: e instanceof Error ? e.message : String(e) }) });
    } finally { setExporting(false); }
  };

  const header = (
    <div className="cmp-h">
      <Tooltip label={t('compare.back')} shortcut="Esc"><button type="button" className="cmp-back" aria-label={t('compare.back')} onClick={close}><Icon name="back" size={14} /></button></Tooltip>
      <h1 ref={title} tabIndex={-1}>{t('compare.title')} · {data?.faces.length ?? faces.length}</h1>
      <div className="cmp-tog" role="group" aria-label={`${t('compare.sameSize')} / ${t('compare.matchXh')}`}>
        <Tooltip label={`${t('compare.sameSize')} — ${t('compare.sameSize.gloss')}`}><button type="button" aria-pressed={!fit} onClick={() => setFit(false)}>{t('compare.sameSize')}</button></Tooltip>
        <Tooltip label={`${t('compare.matchXh')} — ${t('compare.matchXh.gloss')}`}><button type="button" aria-pressed={fit} onClick={() => setFit(true)}>{t('compare.matchXh')}</button></Tooltip>
      </div>
      <div className="cmp-tog"><Tooltip label={`${t('compare.overlay')} — ${t('compare.overlay.gloss')}`}><button type="button" aria-pressed={overlay} onClick={() => setOverlay(v => !v)}>{t('compare.overlay')}</button></Tooltip></div>
      <span className="cmp-sp" />
      <button type="button" className="cmp-btn" disabled={!data || exporting} onClick={() => void exportPng()}><Icon name="plus" size={13} />{t('compare.png')}</button>
      <Tooltip label={t('compare.pdfSoon')}><button type="button" className="cmp-btn" aria-disabled="true" onClick={e => e.preventDefault()}>{t('compare.pdf')}</button></Tooltip>
    </div>
  );

  if (error) return <div className="cmp">{header}<div role="alert" className="cmp-state">{t('compare.error', { msg: error })} <button type="button" onClick={() => setAttempt(n => n + 1)}>{t('compare.retry')}</button></div></div>;
  if (!data) return <div className="cmp">{header}<div role="status" className="cmp-state">{t('compare.loading')}</div></div>;

  const base = data.faces[0];
  const cols = data.faces.length + 1;
  const scale = (f: CompareFace) => (fit && f !== base ? fitScale(base.metrics.x_height_em?.value ?? null, f.metrics.x_height_em?.value ?? null) : 1);
  const cells = (fn: (f: CompareFace) => React.ReactNode) => data.faces.map(f => <div key={keyOf(f.face)} role="cell">{fn(f)}</div>);
  return (
    <div className="cmp">
      {header}
      {data.failed.length > 0 && <div role="alert" className="cmp-state warn">{t('compare.skipped', { n: data.failed.length, reason: data.failed.map(x => x.reason).join('; ') })}</div>}
      <div className="cmp-body">
        <div className="cmp-tbl" role="table" aria-label={t('compare.title')} style={{ ['--n' as string]: data.faces.length }}>
          {data.faces.map(f => <FamilyTap key={keyOf(f.face)} api={api} face={f} into={families.current} />)}
          <Row>
            <div className="rh" role="columnheader" />
            {data.faces.map(f => <div key={keyOf(f.face)} role="columnheader" className={`ch${f === base ? ' base' : ''}`} title={`${f.row.family} ${f.row.subfamily ?? ''}`.trim()}>
              <div className="t">{f === base && <Icon name="anchor" size={13} />}<span>{f.row.family}</span></div>
              <div className="m">{[f === base ? t('compare.base') : null, f.row.subfamily].filter(Boolean).join(' · ')}</div>
            </div>)}
          </Row>

          <Group k="specimen" cols={cols} />
          <Row><div className="rh" role="rowheader"><b>{t('compare.row.line')}</b></div>{cells(f => <SpecimenCell api={api} face={f} text={sample || 'Hxg'} size={SPEC_SIZE * scale(f)} fit={fit && f !== base} />)}</Row>
          {overlay && <Row><div className="rh" role="rowheader"><TermTip term="overlay"><b data-term="overlay">{t('compare.row.overlay')}</b></TermTip><small>{OVERLAY_SPACED}</small></div>{cells(f => <OverlayCell api={api} base={base} face={f} scale={scale(f)} />)}</Row>}
          <Row><div className="rh" role="rowheader"><TermTip term="vi_marks"><b data-term="vi_marks">{t('compare.row.vi')}</b></TermTip><small>Ữ Ặ Ệ Ỗ Ở · ươ</small></div>{cells(f => <ViCell api={api} face={f} size={Math.round(SPEC_SIZE * 1.1 * scale(f))} />)}</Row>

          <Group k="metrics" cols={cols} />
          {COMPARE_METRICS.map(k => <Row key={k}><div className="rh" role="rowheader"><TermTip term={k}><b data-term={termKey(k)}>{label(k)}</b></TermTip><small>{label(k, true)}</small></div>{cells(f => <MetricCell k={k} m={f.metrics[k]} base={base.metrics[k]} isBase={f === base} />)}</Row>)}

          <Group k="class" cols={cols} />
          <Row><div className="rh" role="rowheader"><TermTip term="classification"><b data-term="classification">{t('compare.row.class')}</b></TermTip><small>{gl('compare.row.class.gloss')}</small></div>{cells(f => <ClassCell face={f} />)}</Row>

          <Group k="meta" cols={cols} />
          <Row><div className="rh" role="rowheader"><b>{t('compare.row.license')}</b><small>{gl('compare.row.license.gloss')}</small></div>{cells(f => <LicenseCell face={f} />)}</Row>
          {META_ROWS.map(r => <Row key={r.key}><div className="rh" role="rowheader"><b>{t(r.label)}</b><small>{t(r.gloss)}</small></div>{cells(f => <r.Cell face={f} />)}</Row>)}
        </div>
        <p className="cmp-foot">{t('compare.footnote')} · {t('compare.footnote2')}</p>
      </div>
    </div>
  );
}
