import { defaultSample } from '../../../i18n/sample';
import { useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import type { Api, FaceDetailDto, PairCandidateDto, PairMode, PairRole, PeekOpenResultDto } from '../../../api';
import { t, getLocale } from '../../../i18n';
import { useNoticeStore } from '../../../store/notices';
import { Modal } from '../../Modal';
import '../../desk/desk.css';
import { useUiStore } from '../../../store/ui';
import { useFontFamily } from '../useFontFamily';
import { ZeroState } from '../../zero/ZeroState';
import { faceMenuProps, useFaceActions } from '../../faces/FaceActions';
import { FaceTitle } from '../../ui/FaceTitle';
import { Tooltip } from '../../ui/Tooltip';
import { TermOrTip, termKey } from '../../ui/terms';
import { AXIS_META } from './AnatomyTab';
import { PARAGRAPH } from './SpecimenTab';

const modes = (): Array<[PairMode, string]> => [['safe', t('pair.gate.safe')], ['controlled', t('pair.gate.controlled')], ['experimental', t('pair.gate.experimental')]];

type Props = { api: Api; detail: FaceDetailDto; css: CSSProperties };

export function PairTab({ api, detail, css }: Props): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const setPanelTab = useUiStore(s => s.setPanelTab);
  const faceActions = useFaceActions();
  const [role, setRole] = useState<PairRole>('heading');
  const [mode, setMode] = useState<PairMode>('safe');
  const [projectVi, setProjectVi] = useState(true);
  const [cands, setCands] = useState<PairCandidateDto[]>([]);
  const [idx, setIdx] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [occupied, setOccupied] = useState<PeekOpenResultDto | null>(null);
  const hash = detail.row.hashHex; const index = detail.row.faceIndex;

  useEffect(() => {
    let alive = true;
    setIdx(0); setError(null);
    api.pairCandidates({ hashHex: hash, faceIndex: index }, { role, mode, projectVi, k: 12, collectionId: null, lang: getLocale() }).then(
      c => { if (alive) setCands(c); },
      e => { if (alive) { setCands([]); setError(e?.message ?? String(e)); } },
    );
    return () => { alive = false; };
  }, [api, hash, index, role, mode, projectVi]);

  const cand = cands[Math.min(idx, cands.length - 1)];
  // 2d: mở Pairing Desk với font này làm neo ở vai `role`; ứng viên đang xem thành "thử" (chưa đặt). Vai neo đã có font khác → hỏi rồi gửi lại với replace.
  const openInDesk = async (replace: boolean) => {
    try {
      const res = await api.openCaseFromPeek({ face: { hashHex: hash, faceIndex: index }, as: role === 'body' ? 'body' : 'heading', candidate: cand?.face ?? null, caseId: null, settings: { vi: projectVi, mode }, replace });
      if (res.occupied) { setOccupied(res); return; }
      setOccupied(null);
      useUiStore.getState().openDesk({ caseId: res.case.id, comboId: res.comboId, role: res.otherRole, trial: res.trial });
    } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('peek.openFail', { msg: e instanceof Error ? e.message : String(e) }) }); }
  };
  const anchorRoleName = role === 'heading' ? 'Display' : 'Body';
  const candFamily = useFontFamily(api, cand ? { hashHex: cand.row.hashHex, faceIndex: cand.row.faceIndex, fileSize: cand.row.fileSize } : null);
  const candCss: CSSProperties = { fontFamily: candFamily ? `"${candFamily}", var(--font-ui)` : undefined, fontWeight: cand?.row.weightClass ?? undefined, fontStyle: cand?.row.isItalic ? 'italic' : undefined };
  const [headCss, bodyCss] = role === 'heading' ? [css, candCss] : [candCss, css];
  const modeName = modes().find(([m]) => m === mode)![1];
  const lacksVi = role === 'heading' && projectVi && detail.row.viMissing > 0;

  return (
    <>
      <div className="fa-pair-bar">
        <div className="fa-seg-sm" role="group" aria-label={t('pt.roleGroup')}>
          <button type="button" aria-pressed={role === 'heading'} onClick={() => setRole('heading')}>{t('pt.isHeading')}</button>
          <button type="button" aria-pressed={role === 'body'} onClick={() => setRole('body')}>{t('pt.isBody')}</button>
        </div>
        <div className="fa-seg-sm" role="group" aria-label={t('pt.modeGroup')}>
          {modes().map(([m, name]) => <button key={m} type="button" aria-pressed={mode === m} onClick={() => setMode(m)}>{name}</button>)}
        </div>
        <label className="fa-switch-row">
          <span>{t('pt.projectVi')}</span>
          <button type="button" role="switch" className="fa-switch" aria-checked={projectVi} aria-label={t('pt.projectVi')} onClick={() => setProjectVi(v => !v)} />
        </label>
      </div>

      <button type="button" className="fa-desk-open" data-testid="open-desk" title={t('peek.openDeskTip', { role: anchorRoleName })} onClick={() => void openInDesk(false)}>{t('peek.openDesk')}</button>
      {occupied?.occupied && (
        <Modal title={t('peek.occupied.title', { role: anchorRoleName })} onClose={() => setOccupied(null)}>
          <p>{t('peek.occupied.body', { role: anchorRoleName, old: `${occupied.occupied.row?.family ?? '—'}`, new: detail.row.family })}</p>
          <div className="fa-modal-actions">
            <button type="button" data-testid="occupied-keep" onClick={() => { const r = occupied; setOccupied(null); useUiStore.getState().openDesk({ caseId: r.case.id, comboId: r.comboId, role: r.otherRole, trial: r.trial }); }}>{t('peek.occupied.keep', { old: occupied.occupied.row?.family ?? '—' })}</button>
            <button type="button" className="primary" data-testid="occupied-replace" onClick={() => void openInDesk(true)}>{t('peek.occupied.replace')}</button>
          </div>
        </Modal>
      )}
      {error && <p role="alert" className="fa-soon">{t('desk.candFail', { msg: error })}</p>}

      {!cand && !error && (() => {
        if (lacksVi) return <ZeroState compact glyph="ạ" text={t('pt.lacksVi')} action={{ id: 'see', label: t('pt.seeMissing') }} onAction={() => setPanelTab('vi')} />;
        const nextMode = mode === 'safe' ? 'controlled' : mode === 'controlled' ? 'experimental' : null;
        const nextName = modes().find(([m]) => m === nextMode)?.[1];
        const action = nextMode ? { id: 'mode', label: t('pt.tryMode', { name: nextName ?? '' }) } : projectVi ? { id: 'vi', label: t('pt.turnOffVi') } : undefined;
        return <ZeroState compact glyph="ơ" text={t('pt.noCands', { mode: modeName })} action={action} onAction={id => (id === 'mode' ? setMode(nextMode!) : setProjectVi(false))} />;
      })()}

      {cand && (
        <>
          <div className="fa-pair-nav">
            <button type="button" aria-label={t('pt.prev')} disabled={idx === 0} onClick={() => setIdx(i => Math.max(0, i - 1))}>◀</button>
            <span data-testid="pair-counter">{idx + 1} / {cands.length}</span>
            <button type="button" aria-label={t('pt.next')} disabled={idx >= cands.length - 1} onClick={() => setIdx(i => Math.min(cands.length - 1, i + 1))}>▶</button>
          </div>
          <div className="fa-pair-card" {...faceMenuProps(faceActions, [cand.row])}>
            <div className="fa-pair-heading" data-testid="pair-heading" style={{ ...headCss, fontSize: 26, lineHeight: 1.2 }}>{sample.trim() || defaultSample()}</div>
            <p className="fa-pair-body" data-testid="pair-body" style={{ ...bodyCss, fontSize: 14, lineHeight: 1.55, margin: 0 }}>{PARAGRAPH}</p>
          </div>
          <p className="fa-pair-name" {...faceMenuProps(faceActions, [cand.row])}>
            <FaceTitle api={api} face={cand.row} className="fa-pair-title">{cand.row.family}{cand.row.subfamily ? ` ${cand.row.subfamily}` : ''}</FaceTitle> · {role === 'heading' ? t('pt.asBody') : t('pt.asHeading')}
          </p>
          <p data-testid="pair-summary" className="fa-pair-summary">{cand.summary}</p>
          <ul className="fa-pair-labels">
            {cand.labels.map(l => {
              const axis = AXIS_META[l.axis]?.label ?? l.axis;
              return <TermOrTip key={l.axis} term={termKey(l.axis)} extra={l.relation}><li data-testid="pair-label" aria-label={`${axis}: ${l.relation}`}>{l.symbol} {axis}</li></TermOrTip>;
            })}
          </ul>
          {cand.weightHint && <p className="fa-pair-hint">{cand.weightHint}</p>}
        </>
      )}
    </>
  );
}
