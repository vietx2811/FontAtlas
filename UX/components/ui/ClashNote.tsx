import { t } from '../../i18n';
import { useNameClashRole } from '../../store/nameClash';
import { NoteMark } from './noteMark';

/** G7: dấu "!" giải thích khi font trùng tên PostScript giữa bản Variable (VF) và bản static. Không trùng thì không vẽ gì. */
export function ClashNote({ faceId, inline = false, className }: { faceId: number | undefined; inline?: boolean; className?: string }): React.JSX.Element | null {
  const clash = useNameClashRole(faceId);
  if (!clash) return null;
  const text = t(clash.role === 'static' ? 'clash.static.tip' : clash.exact ? 'clash.vf.tip' : 'clash.vf.tipGuess');
  return <NoteMark kind={clash.role === 'vf' && !clash.exact ? 'info' : 'warn'} inline={inline} className={className} testId="name-clash-note" text={text} />;
}
