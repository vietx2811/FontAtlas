import { useUiStore } from '../../store/ui';
import { t } from '../../i18n';
import type { FaceKeyDto } from '../../api';
import { Tooltip } from '../ui/Tooltip';
import { TermTip } from '../ui/terms';
import './activeRow.css';

type Flow = { toggle(faces: FaceKeyDto[], on: boolean): Promise<void>; setScope(faces: FaceKeyDto[], scope: 'session' | 'persistent'): Promise<void> };

/**
 * Hàng Active trong side peek: công tắc CÓ NHÃN (thuật ngữ Active, có TermTip), dòng trạng thái
 * ("Đang bật · phiên này" / "Luôn bật") và nút đổi phạm vi. Không có face (đang tải) thì công tắc vô hiệu.
 */
export function ActiveRow({ face, flow }: { face: FaceKeyDto | null; flow: Flow }): React.JSX.Element {
  const on = useUiStore(s => (face ? s.active.has(face.hashHex) : false));
  const scope = useUiStore(s => (face ? s.activeScope.get(face.hashHex) ?? 'session' : 'session'));
  const switchLabel = t(on ? 'act.switch.on' : 'act.switch.off');
  const other = scope === 'session' ? 'persistent' : 'session';
  return (
    <div className="fa-act-row" data-testid="active-row">
      <TermTip term="activate"><span className="fa-act-lbl" data-term="activate">{t('act.label')}</span></TermTip>
      <Tooltip label={switchLabel}>
        <button type="button" role="switch" className="fa-switch" aria-checked={on} aria-label={t('act.label')} data-testid="active-switch" aria-disabled={!face || undefined}
          onClick={() => { if (face) void flow.toggle([face], !on); }} />
      </Tooltip>
      <span className="fa-act-state" data-on={on} data-testid="active-status">{on ? t(scope === 'persistent' ? 'act.status.persistent' : 'act.status.session') : t('act.status.off')}</span>
      {on && face && (
        <Tooltip label={t(other === 'persistent' ? 'act.scope.toPersistent.tip' : 'act.scope.toSession.tip')}>
          <button type="button" className="fa-act-scope" data-testid="active-scope" onClick={() => void flow.setScope([face], other)}>
            {t(other === 'persistent' ? 'act.scope.toPersistent' : 'act.scope.toSession')}
          </button>
        </Tooltip>
      )}
    </div>
  );
}
