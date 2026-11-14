import { useUiStore } from '../../store/ui';
import { t } from '../../i18n';
import type { Api, FaceKeyDto } from '../../api';
import { ReasonLine } from './ReasonLine';
import { Tooltip } from '../ui/Tooltip';
import { TermTip } from '../ui/terms';
import { ownerNote } from './conflictModel';
import type { ActivationFlow } from './useActivationFlow';
import './activeRow.css';

/**
 * Hàng Active trong side peek: công tắc CÓ NHÃN (thuật ngữ Active, có TermTip), dòng trạng thái
 * ("Đang bật · phiên này" / "Luôn bật") và nút đổi phạm vi. Không có face (đang tải) thì công tắc vô hiệu.
 * Font "Có sẵn trên máy" (bản giống hệt do bên khác cung cấp): công tắc khoá, dòng giải thích ai cung cấp; Font Atlas không bật gì.
 */
export function ActiveRow({ face, flow, api }: { face: FaceKeyDto | null; flow: ActivationFlow; api?: Api }): React.JSX.Element {
  const on = useUiStore(s => (face ? s.active.has(face.hashHex) : false));
  const scope = useUiStore(s => (face ? s.activeScope.get(face.hashHex) ?? 'session' : 'session'));
  const reasons = useUiStore(s => (face && on ? s.activeReasons.get(face.hashHex) : undefined));
  const avail = useUiStore(s => (face && !s.active.has(face.hashHex) ? s.available.get(face.hashHex) : undefined));
  const who = avail ? ownerNote({ owner: avail.owner, existingPath: avail.path }) : '';
  const switchLabel = t(on ? 'act.switch.on' : 'act.switch.off');
  const other = scope === 'session' ? 'persistent' : 'session';
  return (
    <>
    <div className="fa-act-row" data-testid="active-row" data-available={avail ? 'true' : undefined}>
      <TermTip term="activate"><span className="fa-act-lbl" data-term="activate">{t('act.label')}</span></TermTip>
      <Tooltip label={avail ? t('act.available.why', { who }) : switchLabel}>
        <button type="button" role="switch" className="fa-switch" aria-checked={on} aria-label={t('act.label')} data-testid="active-switch" aria-disabled={!face || !!avail || undefined}
          onClick={() => { if (face && !avail) void flow.toggle([face], !on); }} />
      </Tooltip>
      <span className="fa-act-state" data-on={on} data-available={avail ? 'true' : undefined} data-testid="active-status">
        {avail ? t('act.available.by', { who }) : on ? t(scope === 'persistent' ? 'act.status.persistent' : 'act.status.session') : t('act.status.off')}
      </span>
      {on && face && (
        <Tooltip label={t(other === 'persistent' ? 'act.scope.toPersistent.tip' : 'act.scope.toSession.tip')}>
          <button type="button" className="fa-act-scope" data-testid="active-scope" onClick={() => void flow.setScope([face], other)}>
            {t(other === 'persistent' ? 'act.scope.toPersistent' : 'act.scope.toSession')}
          </button>
        </Tooltip>
      )}
    </div>
    {api && reasons && <ReasonLine api={api} reasons={reasons} />}
    </>
  );
}
