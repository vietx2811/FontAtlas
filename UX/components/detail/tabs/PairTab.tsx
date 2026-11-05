import { useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import type { Api, FaceDetailDto, PairCandidateDto, PairMode, PairRole } from '../../../api';
import { useUiStore } from '../../../store/ui';
import { useFontFamily } from '../useFontFamily';
import { AXIS_META } from './AnatomyTab';
import { PARAGRAPH } from './SpecimenTab';

const MODES: Array<[PairMode, string]> = [['safe', 'An toàn'], ['controlled', 'Có kiểm soát'], ['experimental', 'Thử nghiệm']];

type Props = { api: Api; detail: FaceDetailDto; css: CSSProperties };

export function PairTab({ api, detail, css }: Props): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const setPanelTab = useUiStore(s => s.setPanelTab);
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

      {!cand && !error && (
        lacksVi ? (
          <div className="fa-soon">
            <p>Font tiêu đề này thiếu ký tự tiếng Việt, nên không có ứng viên khi “Dự án dùng tiếng Việt” đang bật.</p>
            <button type="button" className="fa-link" onClick={() => setPanelTab('vi')}>Xem ký tự thiếu</button>
          </div>
        ) : (
          <p className="fa-soon">Không có ứng viên ở chế độ {modeName}. Thử “Có kiểm soát” hoặc tắt “Dự án dùng tiếng Việt”.</p>
        )
      )}

      {cand && (
        <>
          <div className="fa-pair-nav">
            <button type="button" aria-label="Ứng viên trước" disabled={idx === 0} onClick={() => setIdx(i => Math.max(0, i - 1))}>◀</button>
            <span data-testid="pair-counter">{idx + 1} / {cands.length}</span>
            <button type="button" aria-label="Ứng viên sau" disabled={idx >= cands.length - 1} onClick={() => setIdx(i => Math.min(cands.length - 1, i + 1))}>▶</button>
          </div>
          <div className="fa-pair-card">
            <div className="fa-pair-heading" data-testid="pair-heading" style={{ ...headCss, fontSize: 26, lineHeight: 1.2 }}>{sample.trim() || 'Phở bò Hà Nội'}</div>
            <p className="fa-pair-body" data-testid="pair-body" style={{ ...bodyCss, fontSize: 14, lineHeight: 1.55, margin: 0 }}>{PARAGRAPH}</p>
          </div>
          <p className="fa-pair-name">{cand.row.family}{cand.row.subfamily ? ` ${cand.row.subfamily}` : ''} · {role === 'heading' ? 'nội dung' : 'tiêu đề'}</p>
          <p data-testid="pair-summary" className="fa-pair-summary">{cand.summary}</p>
          <ul className="fa-pair-labels">
            {cand.labels.map(l => {
              const axis = AXIS_META[l.axis]?.label ?? l.axis;
              return <li key={l.axis} data-testid="pair-label" title={l.relation} aria-label={`${axis}: ${l.relation}`}>{l.symbol} {axis}</li>;
            })}
          </ul>
          {cand.weightHint && <p className="fa-pair-hint">{cand.weightHint}</p>}
        </>
      )}
    </>
  );
}
