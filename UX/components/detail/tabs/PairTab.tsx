import { useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import type { Api, FaceDetailDto, PairCandidateDto, PairMode, PairRole } from '../../../api';
import { useUiStore } from '../../../store/ui';
import { useFontFamily } from '../useFontFamily';
import { ZeroState } from '../../zero/ZeroState';
import { faceMenuProps, useFaceActions } from '../../faces/FaceActions';
import { FaceTitle } from '../../ui/FaceTitle';
import { Tooltip } from '../../ui/Tooltip';
import { TermOrTip, termKey } from '../../ui/terms';
import { AXIS_META } from './AnatomyTab';
import { PARAGRAPH } from './SpecimenTab';

const MODES: Array<[PairMode, string]> = [['safe', 'An toàn'], ['controlled', 'Có kiểm soát'], ['experimental', 'Thử nghiệm']];

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
  const hash = detail.row.hashHex; const index = detail.row.faceIndex;

  useEffect(() => {
    let alive = true;
    setIdx(0); setError(null);
    api.pairCandidates({ hashHex: hash, faceIndex: index }, { role, mode, projectVi, k: 12, collectionId: null, lang: 'vi' }).then(
      c => { if (alive) setCands(c); },
      e => { if (alive) { setCands([]); setError(e?.message ?? String(e)); } },
    );
    return () => { alive = false; };
  }, [api, hash, index, role, mode, projectVi]);

  const cand = cands[Math.min(idx, cands.length - 1)];
  const candFamily = useFontFamily(api, cand ? { hashHex: cand.row.hashHex, faceIndex: cand.row.faceIndex, fileSize: cand.row.fileSize } : null);
  const candCss: CSSProperties = { fontFamily: candFamily ? `"${candFamily}", var(--font-ui)` : undefined, fontWeight: cand?.row.weightClass ?? undefined, fontStyle: cand?.row.isItalic ? 'italic' : undefined };
  const [headCss, bodyCss] = role === 'heading' ? [css, candCss] : [candCss, css];
  const modeName = MODES.find(([m]) => m === mode)![1];
  const lacksVi = role === 'heading' && projectVi && detail.row.viMissing > 0;

  return (
    <>
      <div className="fa-pair-bar">
        <div className="fa-seg-sm" role="group" aria-label="Vai trò của font này">
          <button type="button" aria-pressed={role === 'heading'} onClick={() => setRole('heading')}>Font này là tiêu đề</button>
          <button type="button" aria-pressed={role === 'body'} onClick={() => setRole('body')}>Font này là nội dung</button>
        </div>
        <div className="fa-seg-sm" role="group" aria-label="Chế độ ghép">
          {MODES.map(([m, name]) => <button key={m} type="button" aria-pressed={mode === m} onClick={() => setMode(m)}>{name}</button>)}
        </div>
        <label className="fa-switch-row">
          <span>Dự án dùng tiếng Việt</span>
          <button type="button" role="switch" className="fa-switch" aria-checked={projectVi} aria-label="Dự án dùng tiếng Việt" onClick={() => setProjectVi(v => !v)} />
        </label>
      </div>

      {error && <p role="alert" className="fa-soon">Không tải được ứng viên: {error}</p>}

      {!cand && !error && (() => {
        if (lacksVi) return <ZeroState compact glyph="ạ" text="Font tiêu đề này thiếu ký tự tiếng Việt" action={{ id: 'see', label: 'Xem ký tự thiếu' }} onAction={() => setPanelTab('vi')} />;
        const nextMode = mode === 'safe' ? 'controlled' : mode === 'controlled' ? 'experimental' : null;
        const nextName = MODES.find(([m]) => m === nextMode)?.[1];
        const action = nextMode ? { id: 'mode', label: `Thử ${nextName}` } : projectVi ? { id: 'vi', label: 'Tắt dự án tiếng Việt' } : undefined;
        return <ZeroState compact glyph="ơ" text={`Không có ứng viên ở chế độ ${modeName}`} action={action} onAction={id => (id === 'mode' ? setMode(nextMode!) : setProjectVi(false))} />;
      })()}

      {cand && (
        <>
          <div className="fa-pair-nav">
            <button type="button" aria-label="Ứng viên trước" disabled={idx === 0} onClick={() => setIdx(i => Math.max(0, i - 1))}>◀</button>
            <span data-testid="pair-counter">{idx + 1} / {cands.length}</span>
            <button type="button" aria-label="Ứng viên sau" disabled={idx >= cands.length - 1} onClick={() => setIdx(i => Math.min(cands.length - 1, i + 1))}>▶</button>
          </div>
          <div className="fa-pair-card" {...faceMenuProps(faceActions, [cand.row])}>
            <div className="fa-pair-heading" data-testid="pair-heading" style={{ ...headCss, fontSize: 26, lineHeight: 1.2 }}>{sample.trim() || 'Phở bò Hà Nội'}</div>
            <p className="fa-pair-body" data-testid="pair-body" style={{ ...bodyCss, fontSize: 14, lineHeight: 1.55, margin: 0 }}>{PARAGRAPH}</p>
          </div>
          <p className="fa-pair-name" {...faceMenuProps(faceActions, [cand.row])}>
            <FaceTitle api={api} face={cand.row} className="fa-pair-title">{cand.row.family}{cand.row.subfamily ? ` ${cand.row.subfamily}` : ''}</FaceTitle> · {role === 'heading' ? 'nội dung' : 'tiêu đề'}
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
