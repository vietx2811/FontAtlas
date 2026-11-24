import { t } from '../../i18n';
import type { FaceKeyDto } from '../../api';
import { useDeriveStore } from '../../store/deriveCopy';
import { useNameClashRole } from '../../store/nameClash';
import { NoteMark } from './noteMark';

/** G7: dấu "!" giải thích khi font trùng tên PostScript giữa bản Variable (VF) và bản static. Không trùng thì không vẽ gì. */
export function ClashNote({ faceId, inline = false, className, face, label }: { faceId: number | undefined; inline?: boolean; className?: string; /** Khoá của font VF: có thì bấm dấu "!" mở "Tạo bản sao đổi tên…" (G7b). */ face?: FaceKeyDto; label?: string }): React.JSX.Element | null {
  const clash = useNameClashRole(faceId);
  if (!clash) return null;
  const canCopy = clash.role === 'vf' && clash.exact && !!face;
  const text = t(clash.role === 'static' ? 'clash.static.tip' : clash.exact ? 'clash.vf.tip' : 'clash.vf.tipGuess');
  return <NoteMark kind={clash.role === 'vf' && !clash.exact ? 'info' : 'warn'} inline={inline} className={className} testId="name-clash-note" text={canCopy ? `${text} ${t('copy.hint')}` : text} onClick={canCopy ? () => useDeriveStore.getState().open(face!, label ?? t('copy.thisFont')) : undefined} />;
}
