// Bảng "Phím tắt & tính năng chính": lưới thẻ gọn, dùng ở cuối hướng dẫn (trước màn kết) và mở lại bằng phím ?.
import { Modal } from '../Modal';
import { t } from '../../i18n';
import { SHORTCUTS } from './shortcuts';
import './shortcuts.css';

export function ShortcutsOverlay({ onClose, primary }: { onClose(): void; primary: string }): React.JSX.Element {
  return (
    <Modal title={t('tips.title')} onClose={onClose} className="fa-tips">
      <p>{t('tips.sub')}</p>
      <ul className="fa-tips-grid" aria-label={t('tips.title')}>
        {SHORTCUTS.map(s => (
          <li key={s.id} data-testid="tip-card" data-tip={s.id}>
            <span className="keys" aria-hidden>{s.keys.map((k, i) => <kbd key={i}>{k}</kbd>)}</span>
            <b>{t(s.title)}</b>
            <span className="d">{t(s.desc)}</span>
            <span className="sr-only">{s.keys.join(' ')}</span>
          </li>
        ))}
      </ul>
      <div className="fa-modal-actions"><button type="button" className="primary" onClick={onClose}>{primary}</button></div>
    </Modal>
  );
}
