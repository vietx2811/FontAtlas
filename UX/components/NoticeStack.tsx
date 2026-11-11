import { useEffect } from 'react';
import { useNoticeStore, type NoticeStore } from '../store/notices';
import { t } from '../i18n';

export const NOTICE_TTL_MS = 8000;

export function NoticeStack({ store = useNoticeStore }: { store?: NoticeStore }): React.JSX.Element | null {
  const notices = store(s => s.notices);
  const dismiss = store(s => s.dismiss);
  // Thông báo thường tự ẩn; lỗi ở lại tới khi đóng.
  useEffect(() => {
    const timers = notices.filter(n => n.kind === 'info').map(n => setTimeout(() => dismiss(n.id), NOTICE_TTL_MS));
    return () => timers.forEach(clearTimeout);
  }, [notices, dismiss]);
  if (!notices.length) return null;
  return (
    <div className="fa-notices">
      {notices.map(n => (
        <div key={n.id} className={`fa-toast ${n.kind}`} role={n.kind === 'error' ? 'alert' : 'status'}>
          <span>{n.text}</span>
          {n.action && <button type="button" onClick={() => { n.action!.run(); dismiss(n.id); }}>{n.action.label}</button>}
          <button type="button" aria-label={t('notice.dismiss')} onClick={() => dismiss(n.id)}>×</button>
        </div>
      ))}
    </div>
  );
}
