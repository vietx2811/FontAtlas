import { t } from '../../i18n';
import { TermTip } from './terms';
import './terms.css';

/** Nhãn "Beta" (tính năng thử nghiệm) kèm thẻ giải thích. Khoá riêng `term.beta.*`; KHÔNG liên quan `pick.gate.experimental` (nhãn cổng của ứng viên). */
export function BetaBadge(): React.JSX.Element {
  return <TermTip term="beta"><span className="fa-beta" data-testid="beta-badge" data-term="beta" tabIndex={0}>{t('term.beta.label')}</span></TermTip>;
}
