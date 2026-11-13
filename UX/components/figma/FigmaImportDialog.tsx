import { useEffect, useMemo, useState } from 'react';
import type { Api } from '../../api';
import type { FigmaImportDto } from '../../bindings';
import { hasKey, t, useLocale, type I18nKey } from '../../i18n';
import { useFigmaImport } from '../../store/figmaImport';
import { useNoticeStore } from '../../store/notices';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { buildRows, choose, counts, selectedFaces, toggle, type ImportRow } from './figmaImport';
import './figma.css';

/** Mã lỗi backend → câu theo ngôn ngữ giao diện (backend không trả câu chữ cho các mã này). */
const codeOf = (e: unknown): string => (typeof e === 'object' && e !== null && 'code' in e ? String((e as { code: unknown }).code) : '') || 'internal';

export function importErrorText(e: unknown): string {
  const code = typeof e === 'object' && e !== null && 'code' in e ? String((e as { code: unknown }).code) : '';
  const key = `err.${code}`;
  return hasKey(key) ? t(key as I18nKey) : t('err.generic', { code: code || 'internal' });
}

const caseName = (file: string | null) => (file ? t('figma.caseName', { file }) : t('figma.caseNameNoFile'));

/** B6: nhập danh sách font từ Figma. Chỉ đọc cho tới khi bấm "Tạo Case"; không bao giờ kích hoạt hay xoá gì. */
export function FigmaImportDialog({ api }: { api: Api }): React.JSX.Element | null {
  useLocale(); // đổi ngôn ngữ giữa chừng: dựng lại cả câu lỗi
  const { phase, imp, show, close } = useFigmaImport();
  const [text, setText] = useState('');
  const [errCode, setErrCode] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rows, setRows] = useState<ImportRow[] | null>(null);
  const [omitFile, setOmitFile] = useState(false);

  useEffect(() => { if (phase === 'paste') { setText(''); setErrCode(null); } if (phase !== 'preview') { setRows(null); setOmitFile(false); } }, [phase]);
  useEffect(() => {
    if (phase !== 'preview' || !imp) return;
    let dead = false;
    setRows(null); setErrCode(null);
    api.resolveFonts(imp.items.map(i => ({ family: i.family, style: i.style })))
      .then(m => { if (!dead) setRows(buildRows(imp, m)); })
      .catch(e => { if (!dead) setErrCode(codeOf(e)); });
    return () => { dead = true; };
  }, [api, phase, imp]);

  const c = useMemo(() => (rows ? counts(rows) : null), [rows]);
  if (phase === 'closed') return null;

  const read = async () => {
    setBusy(true); setErrCode(null);
    try { show(await api.parseFigmaImport(text) as FigmaImportDto); } catch (e) { setErrCode(codeOf(e)); } finally { setBusy(false); }
  };
  const create = async () => {
    if (!rows || !imp) return;
    const faces = selectedFaces(rows);
    if (!faces.length) return;
    setBusy(true);
    try {
      const name = caseName(omitFile ? null : imp.file);
      const made = await api.createCase(name);
      await api.addToCase(made.id, faces, 'added');
      useNoticeStore.getState().push({ kind: 'info', text: t('figma.done', { name, n: faces.length }) });
      window.dispatchEvent(new Event('focus')); // thanh bên nạp lại Case khi cửa sổ lấy focus (xem Sidebar)
      close();
    } catch (e) {
      useNoticeStore.getState().push({ kind: 'error', text: t('figma.fail', { msg: importErrorText(e) }) });
    } finally { setBusy(false); }
  };

  if (phase === 'paste') {
    return (
      <Modal title={t('figma.title')} onClose={close} className="fa-figma">
        <p>{t('figma.pasteHint')}</p>
        <textarea className="fa-figma-text" aria-label={t('figma.pasteLabel')} value={text} onChange={e => setText(e.target.value)} spellCheck={false} rows={6} />
        {errCode && <p className="fa-figma-err" role="alert">{importErrorText({ code: errCode })}</p>}
        <div className="fa-modal-actions">
          <button type="button" onClick={close}>{t('figma.cancel')}</button>
          <button type="button" className="primary" disabled={busy || !text.trim()} onClick={() => void read()}>{t('figma.read')}</button>
        </div>
      </Modal>
    );
  }

  const n = rows ? selectedFaces(rows).length : 0;
  return (
    <Modal title={t('figma.title')} onClose={close} className="fa-figma">
      {errCode && <p className="fa-figma-err" role="alert">{importErrorText({ code: errCode })}</p>}
      {!rows && !errCode && <p role="status">{t('figma.resolving')}</p>}
      {rows && c && imp && <>
        <p role="status">{t('figma.summary', { n: c.total, exact: c.exact, ambiguous: c.ambiguous, rest: c.familyOnly + c.none })}</p>
        {imp.file && <label className="fa-figma-file"><input type="checkbox" checked={omitFile} onChange={e => setOmitFile(e.target.checked)} />{t('figma.omitFile')}</label>}
        <ul className="fa-figma-list" aria-label={t('figma.title')}>
          {rows.map(r => (
            <li key={r.index} className="fa-figma-row fa-note-host" data-status={r.status}>
              <input type="checkbox" aria-label={t('figma.include', { family: r.family, style: r.style })} checked={r.include} disabled={!r.pick} onChange={() => setRows(toggle(rows, r.index))} />
              <div className="fa-figma-name"><strong>{r.family}</strong> <span>{r.style}</span></div>
              <div className="fa-figma-state">
                {r.status === 'ambiguous'
                  ? <select aria-label={t('figma.pick', { family: r.family, style: r.style })} value={r.pick ? `${r.pick.hashHex}:${r.pick.faceIndex}` : ''}
                      onChange={e => { const [h, i] = e.target.value.split(':'); setRows(choose(rows, r.index, e.target.value ? { hashHex: h, faceIndex: Number(i) } : null)); }}>
                      <option value="">{t('figma.pick.none')}</option>
                      {r.faces.map((f, k) => <option key={`${f.face.hashHex}:${f.face.faceIndex}`} value={`${f.face.hashHex}:${f.face.faceIndex}`}>{`${f.family} ${f.style} (${k + 1})`}</option>)}
                    </select>
                  : <span>{t(`figma.st.${r.status}` as I18nKey, { style: r.style })}</span>}
              </div>
              {r.figmaMissing && <NoteMark inline text={t('figma.figmaMissing')} />}
              {r.pick && r.faces.find(f => f.face.hashHex === r.pick?.hashHex && f.face.faceIndex === r.pick?.faceIndex)?.missing && <NoteMark inline text={t('figma.fileMissing')} />}
            </li>
          ))}
        </ul>
      </>}
      <div className="fa-modal-actions">
        <button type="button" onClick={close}>{t('figma.cancel')}</button>
        <button type="button" className="primary" disabled={busy || n === 0} title={n === 0 ? t('figma.create.none') : undefined} onClick={() => void create()}>{t('figma.create', { n })}</button>
      </div>
    </Modal>
  );
}
