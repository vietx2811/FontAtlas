import type { ModelUi } from '../../store/model';
import './modelFeedback.css';
import { t } from '../../i18n';

export function ModelSpinner({ ui }: { ui: ModelUi }): React.JSX.Element | null {
  return ui.code === 'E2' && ui.progress === null ? <span data-testid="model-spinner" className="fa-model-spinner" aria-hidden /> : null;
}

export function ModelDetails({ detail }: { detail?: string }): React.JSX.Element | null {
  return detail ? <details className="fa-model-details"><summary>{t('model.details')}</summary><p>{detail}</p></details> : null;
}
