import { Fragment, useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { COMPARE_METRICS, type CompareDto, type CompareFace } from '../../api/compare';
import { hasKey, t, type I18nKey } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { COMPARE_MAX, COMPARE_MIN, useUiStore } from '../../store/ui';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import { TermTip, termKey, termName } from '../ui/terms';
import { ClassCell, MetricCell, OVERLAY_SPACED, OverlayCell, SAMPLE_VI_GLOSS, SpecimenCell, ViCell, WithFamily } from './CompareCells';
import { Menu } from '../ui/Menu';
import { FacePicker } from './FacePicker';
import { GROUPS, useRowConfig, type GroupKey } from './rowConfig';
import { META_ROWS, LicenseCell } from './MetaCells';
import { fitScale } from './compareMath';
import { ensureFonts, renderCompare, renderCompareBlob } from './exportPng';
import './compare.css';

type RowDef = { id: string; group: GroupKey; name: string; head: React.ReactNode; cell(f: CompareFace): React.ReactNode };
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
/** Mỗi hàng dữ liệu có thêm một ô đệm ở cuối cho cột "+" hẹp (hàng nhóm trải hết bề ngang nên không cần). */
const Row = ({ children, group }: { children: React.ReactNode; group?: boolean }) => <div className="row" role="row">{children}{!group && <div role="cell" className="addc" />}</div>;
const GroupHead = ({ k, cols, collapsed, onToggle }: { k: GroupKey; cols: number; collapsed: boolean; onToggle(): void }) => (
  <Row group>
    <div role="columnheader" aria-colspan={cols} className="gh" data-group={k} data-collapsed={collapsed}>
      <span className="ghin">
        <Tooltip label={collapsed ? t('compare.group.expand') : t('compare.group.collapse')}>
          <button type="button" className="cmp-ib" aria-expanded={!collapsed} aria-label={`${collapsed ? t('compare.group.expand') : t('compare.group.collapse')} · ${t(`compare.sec.${k}` as I18nKey)}`} onClick={onToggle}><Icon name={collapsed ? 'plus' : 'minus'} size={12} /></button>
        </Tooltip>
        {t(`compare.sec.${k}` as I18nKey)}{gl(`compare.sec.${k}.gloss`) && <small> · {gl(`compare.sec.${k}.gloss`)}</small>}
      </span>
    </div>
  </Row>
);

function ColHead({ face, base, pinned, canRemove }: { face: CompareFace; base: boolean; pinned: boolean; canRemove: boolean }): React.JSX.Element {
  const id = keyOf(face.face);
  const name = `${face.row.family} ${face.row.subfamily ?? ''}`.trim();
  return (
    <div role="columnheader" className={`ch${base ? ' base' : ''}${pinned ? ' pinned' : ''}`} title={name} data-pinned={pinned}>
      <div className="t">{base && <Icon name="anchor" size={13} />}<span>{face.row.family}</span>
        <span className="act">
          <Tooltip label={pinned ? t('compare.unpin.tip') : t('compare.pin.tip')}>
            <button type="button" className="cmp-ib" aria-pressed={pinned} aria-label={`${pinned ? t('compare.unpin') : t('compare.pin')} · ${name}`} onClick={() => useUiStore.getState().toggleComparePin(id)}><Icon name="pin" size={13} /></button>
          </Tooltip>
          <Tooltip label={canRemove ? t('compare.remove') : t('compare.remove.min', { n: COMPARE_MIN })}>
            <button type="button" className="cmp-ib" aria-label={`${t('compare.remove')} · ${name}`} aria-disabled={canRemove ? undefined : 'true'} onClick={() => { if (canRemove) useUiStore.getState().removeCompareFace(id); }}><Icon name="x" size={13} /></button>
          </Tooltip>
        </span>
      </div>
      <div className="m">{[base ? t('compare.base') : null, face.row.subfamily].filter(Boolean).join(' · ')}</div>
    </div>
  );
}

/** Màn Compare: 2–8 cột, cột đầu là mốc; toàn bộ số đo là số thật từ phân tích (không có thì ghi "Không đo được"). */
export function ComparePage({ api, faces }: { api: Api; faces: FaceKeyDto[] }): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const [data, setData] = useState<CompareDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [fit, setFit] = useState(false);
  const [overlay, setOverlay] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [picker, setPicker] = useState<DOMRect | null>(null);
  const session = useUiStore(s => s.compareSession);
  const rc = useRowConfig();
  const [rowMenu, setRowMenu] = useState<{ group: GroupKey; rect: DOMRect } | null>(null);
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
      const loaded = await ensureFonts(families.current.values(), sample);
      const r = renderCompare({ data, families: families.current, sample, fit, dark: isDark(), overlay, loaded });
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
  const pins = session?.pinned ?? [];
  const order = (f: CompareFace) => { const i = pins.indexOf(keyOf(f.face)); return i < 0 ? pins.length + data.faces.indexOf(f) : i; };
  const shown = [...data.faces].sort((a, b) => order(a) - order(b));
  const full = shown.length >= COMPARE_MAX;
  const cols = shown.length + 2;
  const scale = (f: CompareFace) => (fit && f !== base ? fitScale(base.metrics.x_height_em?.value ?? null, f.metrics.x_height_em?.value ?? null) : 1);
  const cells = (fn: (f: CompareFace) => React.ReactNode) => shown.map(f => <div key={keyOf(f.face)} role="cell">{fn(f)}</div>);
  const defs: RowDef[] = [
    { id: 'specimen.line', group: 'specimen', name: t('compare.row.line'), head: <b>{t('compare.row.line')}</b>, cell: f => <SpecimenCell api={api} face={f} text={sample || 'Hxg'} size={SPEC_SIZE * scale(f)} fit={fit && f !== base} /> },
    ...(overlay ? [{ id: 'specimen.overlay', group: 'specimen' as const, name: t('compare.row.overlay'), head: <><TermTip term="overlay"><b data-term="overlay">{t('compare.row.overlay')}</b></TermTip><small>{OVERLAY_SPACED}</small></>, cell: (f: CompareFace) => <OverlayCell api={api} base={base} face={f} scale={scale(f)} /> }] : []),
    { id: 'specimen.vi', group: 'specimen', name: t('compare.row.vi'), head: <><TermTip term="vi_marks"><b data-term="vi_marks">{t('compare.row.vi')}</b></TermTip><small>{SAMPLE_VI_GLOSS}</small></>, cell: f => <ViCell api={api} face={f} size={Math.round(SPEC_SIZE * 1.1 * scale(f))} /> },
    ...COMPARE_METRICS.map((k): RowDef => ({ id: `metrics.${k}`, group: 'metrics', name: label(k), head: <><TermTip term={k}><b data-term={termKey(k)}>{label(k)}</b></TermTip><small>{label(k, true)}</small></>, cell: f => <MetricCell k={k} m={f.metrics[k]} base={base.metrics[k]} isBase={f === base} /> })),
    { id: 'class.class', group: 'class', name: t('compare.row.class'), head: <><TermTip term="classification"><b data-term="classification">{t('compare.row.class')}</b></TermTip><small>{gl('compare.row.class.gloss')}</small></>, cell: f => <ClassCell face={f} /> },
    { id: 'meta.license', group: 'meta', name: t('compare.row.license'), head: <><b>{t('compare.row.license')}</b><small>{gl('compare.row.license.gloss')}</small></>, cell: f => <LicenseCell face={f} /> },
    ...META_ROWS.map((r): RowDef => ({ id: `meta.${r.key}`, group: 'meta', name: t(r.label), head: <><b>{t(r.label)}</b><small>{t(r.gloss)}</small></>, cell: f => <r.Cell face={f} /> })),
  ];
  return (
    <div className="cmp">
      {header}
      {data.failed.length > 0 && <div role="alert" className="cmp-state warn">{t('compare.skipped', { n: data.failed.length, reason: data.failed.map(x => x.reason).join('; ') })}</div>}
      <div className="cmp-body">
        <div className="cmp-tbl" role="table" aria-label={t('compare.title')} style={{ ['--n' as string]: shown.length }}>
          {data.faces.map(f => <FamilyTap key={keyOf(f.face)} api={api} face={f} into={families.current} />)}
          <Row group>
            <div className="rh" role="columnheader" />
            {shown.map(f => <ColHead key={keyOf(f.face)} face={f} base={f === base} pinned={pins.includes(keyOf(f.face))} canRemove={shown.length > COMPARE_MIN} />)}
            <div role="columnheader" className="addh">
              <Tooltip label={full ? t('compare.add.full', { n: COMPARE_MAX }) : t('compare.add.tip')}>
                <button type="button" className="cmp-add" aria-label={t('compare.add')} aria-disabled={full ? 'true' : undefined} aria-haspopup="dialog" aria-expanded={picker !== null}
                  onClick={e => { if (full) return; setPicker(e.currentTarget.getBoundingClientRect()); }}><Icon name="plus" size={14} /></button>
              </Tooltip>
            </div>
          </Row>

          {GROUPS.map(g => {
            const inGroup = defs.filter(d => d.group === g);
            const hiddenRows = inGroup.filter(d => rc.config.hidden.includes(d.id));
            const collapsed = rc.config.collapsed.includes(g);
            return (
              <Fragment key={g}>
                <GroupHead k={g} cols={cols} collapsed={collapsed} onToggle={() => rc.toggleGroup(g)} />
                {!collapsed && inGroup.filter(d => !rc.config.hidden.includes(d.id)).map(d => (
                  <Row key={d.id}>
                    <div className="rh" role="rowheader" data-row={d.id}>{d.head}
                      <Tooltip label={t('compare.row.hide')}><button type="button" className="cmp-hide" aria-label={`${t('compare.row.hide')} · ${d.name}`} onClick={() => rc.hideRow(d.id)}><Icon name="x" size={12} /></button></Tooltip>
                    </div>
                    {cells(d.cell)}
                  </Row>
                ))}
                {!collapsed && hiddenRows.length > 0 && (
                  <Row group><div role="cell" aria-colspan={cols} className="gadd">
                    <Tooltip label={t('compare.row.add.tip')}><button type="button" className="cmp-ib" aria-label={`${t('compare.row.add')} · ${t(`compare.sec.${g}` as I18nKey)}`} aria-haspopup="menu" onClick={e => setRowMenu({ group: g, rect: e.currentTarget.getBoundingClientRect() })}><Icon name="plus" size={13} /></button></Tooltip>
                  </div></Row>
                )}
              </Fragment>
            );
          })}
        </div>
        {rowMenu && <Menu label={t('compare.row.menu')} anchor={{ rect: rowMenu.rect }} onClose={() => setRowMenu(null)}
          entries={defs.filter(d => d.group === rowMenu.group && rc.config.hidden.includes(d.id)).map(d => ({ id: d.id, label: d.name, onSelect: () => rc.showRow(d.id) }))} />}
        {picker && <FacePicker api={api} anchor={picker} exclude={new Set(data.faces.map(f => keyOf(f.face)))} onClose={() => setPicker(null)} onPick={r => { useUiStore.getState().addCompareFace({ hashHex: r.hashHex, faceIndex: r.faceIndex }); setPicker(null); }} />}
        <p className="cmp-foot">{t('compare.footnote')} · {t('compare.footnote2')}</p>
      </div>
    </div>
  );
}
