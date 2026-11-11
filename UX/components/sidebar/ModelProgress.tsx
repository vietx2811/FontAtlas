import { useEffect, useRef, useState } from 'react';
import type { Api } from '../../api';
import { useModelStore, useModelUi, type ModelStore } from '../../store/model';
import './modelProgress.css';
import { ModelDetails, ModelSpinner } from '../model/ModelFeedback';
import { Tooltip } from '../ui/Tooltip';
import { t } from '../../i18n';

export const SR_THROTTLE_MS = 10_000;

/** Văn bản cho trình đọc màn hình: đổi ngay khi sang trạng thái khác, còn cùng trạng thái thì tối đa một lần mỗi 10 s. */
function useThrottledAnnouncement(text: string | null, code: string): string {
  const [out, setOut] = useState(text ?? '');
  const last = useRef({ t: Date.now(), code });
  const pending = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => {
    const now = Date.now();
    const sameCode = last.current.code === code;
    const wait = sameCode ? SR_THROTTLE_MS - (now - last.current.t) : 0;
    clearTimeout(pending.current);
    const commit = () => { last.current = { t: Date.now(), code }; setOut(text ?? ''); };
    if (wait <= 0) commit(); else pending.current = setTimeout(commit, wait);
    return () => clearTimeout(pending.current);
  }, [text, code]);
  return out;
}

function Ring({ value }: { value: number }): React.JSX.Element {
  const C = 2 * Math.PI * 7;
  return (
    <svg data-ring className="fa-ring" viewBox="0 0 17 17" width="17" height="17" aria-hidden>
      <circle className="bgc" cx="8.5" cy="8.5" r="7" />
      <circle className="fgc" cx="8.5" cy="8.5" r="7" strokeDasharray={C} strokeDashoffset={C * (1 - Math.min(1, Math.max(0, value)))} />
    </svg>
  );
}

export function ModelProgress({ api, store = useModelStore }: { api: Api; store?: ModelStore }): React.JSX.Element | null {
  const ui = useModelUi(store);
  const totalBytes = store(s => s.model?.totalBytes);
  const announced = useThrottledAnnouncement(ui.text, `${ui.code}:${!!ui.checking}`);
  if (!ui.text) return null;
  const error = ui.severity === 'error';
  return (
    <Tooltip label={ui.detail} disabled={ui.code !== 'E8' && ui.code !== 'E9'}>
    <div className={`fa-model${error ? ' err' : ''}`} data-testid="model-progress" data-code={ui.code}>
      {ui.progress !== null ? <Ring key={`${ui.code}:${totalBytes}`} value={ui.progress} /> : ui.code === 'E2' ? <ModelSpinner ui={ui} /> : <span className="fa-model-dot" aria-hidden>{error ? '⚠' : 'ⓘ'}</span>}
      <span className="fa-model-text">{ui.text}</span>
      <span className="fa-model-actions">
        {ui.actions.map(a => (
          <button key={a.label} type="button" aria-label={a.label} onClick={() => void store.getState().act(api, a.id)}>{a.label}</button>
        ))}
        {ui.code === 'E1' && <Tooltip label={t('model.dismissHint')}><button type="button" aria-label={t('model.dismissHint')} onClick={() => store.getState().dismissE1()}>×</button></Tooltip>}
      </span>
      <ModelDetails detail={ui.detail} />
      {/* Vùng đọc màn hình: lỗi dùng assertive, tiến độ dùng polite (≤ 1 lần/10 s). */}
      <span className="sr-only" data-testid="model-live" role={error ? 'alert' : 'status'} aria-live={error ? 'assertive' : 'polite'}>{announced}</span>
    </div>
    </Tooltip>
  );
}
