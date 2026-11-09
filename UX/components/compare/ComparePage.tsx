import { Fragment, useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { COMPARE_METRICS, type CompareDto, type CompareFace } from '../../api/compare';
import { hasKey, t, type I18nKey } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { COMPARE_MAX, COMPARE_MIN, OVERLAY_MAX, useUiStore } from '../../store/ui';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import { TermTip, termKey, termName } from '../ui/terms';
import { ClassCell, MetricCell, SAMPLE_VI, OverlayPanel, SpecimenCell, ViCell, WithFamily } from './CompareCells';
import { Menu } from '../ui/Menu';
import { FacePicker } from './FacePicker';
import { GROUPS, useRowConfig, type GroupKey } from './rowConfig';
import { META_ROWS, LicenseCell } from './MetaCells';
import { barScale, fitScale } from './compareMath';
import { detectMissingAll, ensureFonts, renderCompare, renderCompareBlob } from './exportPng';
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

function ColHead({ face, base, pinned, canRemove, overlay }: { face: CompareFace; base: boolean; pinned: boolean; canRemove: boolean; overlay: { on: boolean; full: boolean } | null }): React.JSX.Element {
  const id = keyOf(face.face);
  const name = `${face.row.family} ${face.row.subfamily ?? ''}`.trim();
  return (
    <div role="columnheader" className={`ch${base ? ' base' : ''}${pinned ? ' pinned' : ''}`} title={name} data-pinned={pinned}>
      <div className="t">{base && <Icon name="anchor" size={13} />}<span>{face.row.family}</span>
        <span className="act">
          {overlay && (
            <Tooltip label={overlay.on ? t('compare.ov.col.off') : overlay.full ? t('compare.ov.max', { n: OVERLAY_MAX }) : t('compare.ov.col')}>
              <button type="button" className="cmp-ib" aria-pressed={overlay.on} aria-disabled={!overlay.on && overlay.full ? 'true' : undefined} aria-label={`${t('compare.row.overlay')} · ${name}`} onClick={() => useUiStore.getState().toggleCompareOverlayFace(id)}><Icon name="eye" size={13} /></button>
            </Tooltip>
          )}
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
  const [exporting, setExporting] = useState(false);
  const [picker, setPicker] = useState<DOMRect | null>(null);
  const addBtn = useRef<HTMLButtonElement>(null);
  const closePicker = () => { setPicker(null); addBtn.current?.focus(); };
  const session = useUiStore(s => s.compareSession);
  const overlay = session?.overlayOn ?? false;
  const overlayIds = session?.overlayIds ?? [];
  const rc = useRowConfig();
  const [rowMenu, setRowMenu] = useState<{ group: GroupKey; rect: DOMRect } | null>(null);
  const families = useRef(new Map<string, string | null>());
  const title = useRef<HTMLHeadingElement>(null);
  const close = () => useUiStore.getState().closeCompare();
  const idsKey = faces.map(keyOf).join('|');

  const dataRef = useRef<CompareDto | null>(null);
  dataRef.current = data;
  const lastAttempt = useRef(attempt);
  // Đổi tập cột KHÔNG nháy bảng: bỏ cột (mốc không đổi) lọc tại chỗ; thêm cột chỉ nạp face mới; còn lại mới nạp lại toàn bộ (vẫn giữ bảng cũ trong lúc chờ).
  useEffect(() => {
    let alive = true; setError(null);
    const cur = dataRef.current; const retry = lastAttempt.current !== attempt; lastAttempt.current = attempt;
    const ids = faces.map(keyOf);
    const have = cur ? cur.faces.map(f => keyOf(f.face)) : [];
    const known = new Set([...have, ...(cur?.failed.map(x => keyOf(x.face)) ?? [])]);
    const full = () => { api.compareFaces(faces).then(d => { if (alive) setData(d); }, e => { if (alive) { if (cur) failAdd(ids.filter(k => !known.has(k)), e); else setError(e instanceof Error ? e.message : String(e)); } }); };
    if (!cur || retry || ids[0] !== have[0]) { if (!cur || retry) setData(null); full(); return () => { alive = false; }; }
    const extras = ids.filter(k => !known.has(k));
    if (extras.length === 0) {
      if (ids.length !== have.length) setData({ ...cur, faces: cur.faces.filter(f => ids.includes(keyOf(f.face))) });
      return () => { alive = false; };
    }
    const baseKey = have[0];
    const extraFaces = faces.filter(f => extras.includes(keyOf(f)));
    api.compareFaces([faces.find(f => keyOf(f) === baseKey)!, ...extraFaces]).then(d => {
      if (!alive) return;
      const pool = new Map([...cur.faces, ...d.faces.filter(f => extras.includes(keyOf(f.face)))].map(f => [keyOf(f.face), f] as const));
      setData({ faces: ids.map(k => pool.get(k)).filter((f): f is CompareFace => !!f), failed: [...cur.failed, ...d.failed] });
    }, e => { if (alive) failAdd(extras, e); });
    return () => { alive = false; };
  }, [api, idsKey, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  // Thêm cột thất bại: báo lỗi và bỏ khỏi phiên (không để cột ma kẹt trong session).
  const failAdd = (keys: string[], e: unknown) => {
    useUiStore.getState().pruneCompareFaces(keys);
    useNoticeStore.getState().push({ kind: 'error', text: t('compare.error', { msg: e instanceof Error ? e.message : String(e) }) });
  };
  // Face không đọc được thì không có cột để bỏ: gỡ khỏi phiên (lý do vẫn hiện ở dải cảnh báo).
  useEffect(() => {
    if (!data || data.failed.length === 0) return;
    const sess = useUiStore.getState().compareSession; if (!sess) return;
    const bad = data.failed.map(x => keyOf(x.face)).filter(k => sess.faces.some(f => keyOf(f) === k));
    if (bad.length) useUiStore.getState().pruneCompareFaces(bad);
  }, [data]);

  // Tiêu điểm vào màn khi mở, trả về chỗ cũ khi đóng.
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    title.current?.focus();
    return () => { if (before && before.isConnected) before.focus(); };
  }, []);

  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing || e.keyCode === 229) return; // Esc của bộ gõ (Telex…) không đóng màn
      if ((e.target as HTMLElement | null)?.matches?.('input, textarea, [contenteditable="true"]')) return; // đang gõ: Esc thuộc về ô nhập (nó tự blur)
      if (document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]')) return;
      close();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);

  const missingForPng = (text: string) => detectMissingAll(api, (data?.faces ?? []).map(f => ({ face: f.face, family: families.current.get(keyOf(f.face)) ?? null })), text);
  // PNG đúng như màn hình: thứ tự pin, hàng ẩn, nhóm thu gọn.
  const screenLayout = () => {
    const cs = useUiStore.getState().compareSession;
    const pins = cs?.pinned ?? [];
    const keys = (data?.faces ?? []).map(f => keyOf(f.face));
    const order = [...keys].sort((a, b) => (pins.includes(a) ? pins.indexOf(a) : pins.length + keys.indexOf(a)) - (pins.includes(b) ? pins.indexOf(b) : pins.length + keys.indexOf(b)));
    return { order, hiddenRows: new Set(rc.config.hidden), collapsed: new Set<string>(rc.config.collapsed) };
  };
  const isDark = () => document.documentElement.dataset.theme === 'dark' || (!document.documentElement.dataset.theme && typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches);

  // Móc QA native (chỉ khi chạy kịch bản QA): vẽ PNG như nút Xuất, không mở hộp thoại, trả bản kê nội dung + mẫu điểm ảnh.
  useEffect(() => {
    if (!data || !(window as unknown as { __FA_QA_PICKS__?: unknown }).__FA_QA_PICKS__) return;
    (window as unknown as { __faQaComparePng?: () => Promise<unknown> }).__faQaComparePng = async () => {
      const loaded = await ensureFonts(families.current.values(), sample);
      const missing = await missingForPng(`${sample || 'Hxg'}${SAMPLE_VI}`);
      const r = renderCompare({ data, families: families.current, sample, fit, dark: isDark(), overlay: overlay ? overlayIds : null, loaded, missing, ...screenLayout() });
      if (!r) return null;
      const px = r.canvas.getContext('2d')!.getImageData(2, r.canvas.height - 2, 1, 1).data;
      return { ...r.manifest, cornerPx: [px[0], px[1], px[2]], dataUrl: r.canvas.toDataURL('image/png') };
    };
    return () => { delete (window as unknown as { __faQaComparePng?: unknown }).__faQaComparePng; };
  }, [data, sample, fit, overlay, overlayIds.join('|'), rc.config]); // eslint-disable-line react-hooks/exhaustive-deps

  const exportPng = async () => {
    if (!data || exporting) return;
    setExporting(true);
    try {
      const { blob, name } = await renderCompareBlob({ data, families: families.current, sample, fit, dark: isDark(), overlay: overlay ? overlayIds : null, missing: await missingForPng(`${sample || 'Hxg'}${SAMPLE_VI}`), ...screenLayout() });
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
      <div className="cmp-tog"><Tooltip label={`${t('compare.overlay')} — ${t('compare.overlay.gloss')}`}><button type="button" aria-pressed={overlay} onClick={() => useUiStore.getState().setCompareOverlay(!overlay)}>{t('compare.overlay')}</button></Tooltip></div>
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
  const full = (session?.faces.length ?? shown.length) >= COMPARE_MAX;
  const cols = shown.length + 2;
  const scale = (f: CompareFace) => (fit && f !== base ? fitScale(base.metrics.x_height_em?.value ?? null, f.metrics.x_height_em?.value ?? null) : 1);
  const cells = (fn: (f: CompareFace) => React.ReactNode) => shown.map(f => <div key={keyOf(f.face)} role="cell">{fn(f)}</div>);
  const defs: RowDef[] = [
    { id: 'specimen.line', group: 'specimen', name: t('compare.row.line'), head: <b>{t('compare.row.line')}</b>, cell: f => <SpecimenCell api={api} face={f} text={sample} size={SPEC_SIZE * scale(f)} fit={fit && f !== base} /> },
    { id: 'specimen.vi', group: 'specimen', name: t('compare.row.vi'), head: <><b>{t('compare.row.vi')}</b><small><TermTip term="vi_marks"><span data-term="vi_marks">{termName('vi_marks')}</span></TermTip> · {t('compare.row.vi.gloss')}</small></>, cell: f => <ViCell api={api} face={f} size={Math.round(SPEC_SIZE * 1.1 * scale(f))} /> },
    ...COMPARE_METRICS.map((k): RowDef => ({ id: `metrics.${k}`, group: 'metrics', name: label(k), head: <><TermTip term={k}><b data-term={termKey(k)}>{label(k)}</b></TermTip><small>{label(k, true)}</small></>, cell: f => <MetricCell k={k} m={f.metrics[k]} base={base.metrics[k]} isBase={f === base} scale={barScale(k, shown.map(x => x.metrics[k].value))} /> })),
    { id: 'class.class', group: 'class', name: t('compare.row.class'), head: <><TermTip term="classification"><b data-term="classification">{t('compare.row.class')}</b></TermTip><small>{gl('compare.row.class.gloss')}</small></>, cell: f => <ClassCell face={f} /> },
    { id: 'meta.license', group: 'meta', name: t('compare.row.license'), head: <><b>{t('compare.row.license')}</b><small>{gl('compare.row.license.gloss')}</small></>, cell: f => <LicenseCell face={f} /> },
    ...META_ROWS.map((r): RowDef => ({ id: `meta.${r.key}`, group: 'meta', name: t(r.label), head: <><b>{t(r.label)}</b><small>{t(r.gloss)}</small></>, cell: f => <r.Cell face={f} /> })),
  ];
  return (
    <div className="cmp">
      {header}
      {data.failed.length > 0 && <div role="alert" className="cmp-state warn">{t('compare.skipped', { n: data.failed.length, reason: data.failed.map(x => x.reason).join('; ') })}</div>}
      <div className="cmp-body">
        {overlay && <OverlayPanel api={api} base={base} selected={data.faces.filter(f => overlayIds.includes(keyOf(f.face)))} scale={scale} />}
        <div className="cmp-tbl" role="table" aria-label={t('compare.title')} style={{ ['--n' as string]: shown.length }}>
          {data.faces.map(f => <FamilyTap key={keyOf(f.face)} api={api} face={f} into={families.current} />)}
          <Row group>
            <div className="rh" role="columnheader" />
            {shown.map(f => <ColHead key={keyOf(f.face)} face={f} base={f === base} pinned={pins.includes(keyOf(f.face))} canRemove={shown.length > COMPARE_MIN} overlay={overlay ? { on: overlayIds.includes(keyOf(f.face)), full: overlayIds.length >= OVERLAY_MAX } : null} />)}
            <div role="columnheader" className="addh">
              <Tooltip label={full ? t('compare.add.full', { n: COMPARE_MAX }) : t('compare.add.tip')}>
                <button type="button" className="cmp-add" aria-label={t('compare.add')} aria-disabled={full ? 'true' : undefined} aria-haspopup="dialog" aria-expanded={picker !== null}
                  ref={addBtn} data-picker-trigger onClick={e => { if (full) return; if (picker) closePicker(); else setPicker(e.currentTarget.getBoundingClientRect()); }}><Icon name="plus" size={14} /></button>
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
        {picker && <FacePicker api={api} anchor={picker} exclude={new Set(data.faces.map(f => keyOf(f.face)))} onClose={closePicker} onPick={r => { const ok = useUiStore.getState().addCompareFace({ hashHex: r.hashHex, faceIndex: r.faceIndex }); if (!ok) useNoticeStore.getState().push({ kind: 'error', text: t('compare.add.fail') }); closePicker(); }} />}
        <p className="cmp-foot">{t('compare.footnote')} · {t('compare.footnote2')}</p>
      </div>
    </div>
  );
}
