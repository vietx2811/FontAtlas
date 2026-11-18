import { useEffect, useRef } from 'react';
import { NoteMark } from './ui/noteMark';
import { useNoticeStore, type NoticeStore } from '../store/notices';
import { t } from '../i18n';

export const NOTICE_TTL_MS = 8000;

export function NoticeStack({ store = useNoticeStore }: { store?: NoticeStore }): React.JSX.Element | null {
  const notices = store(s => s.notices);
  const dismiss = store(s => s.dismiss);
  // Thông báo thường tự ẩn sau NOTICE_TTL_MS tính từ lúc nó xuất hiện (không bị thông báo khác đến sau làm lại đồng hồ); lỗi ở lại tới khi đóng.
  const timers = useRef(new Map<number, ReturnType<typeof setTimeout>>());
  useEffect(() => {
    const live = new Set(notices.map(n => n.id));
    for (const [id, tm] of timers.current) if (!live.has(id)) { clearTimeout(tm); timers.current.delete(id); }
    for (const n of notices) if (n.kind === 'info' && !timers.current.has(n.id)) timers.current.set(n.id, setTimeout(() => { timers.current.delete(n.id); dismiss(n.id); }, NOTICE_TTL_MS));
  }, [notices, dismiss]);
  useEffect(() => { const m = timers.current; return () => { m.forEach(clearTimeout); m.clear(); }; }, []);
  if (!notices.length) return null;
  return (
    <div className="fa-notices">
      {notices.map(n => (
        <div key={n.id} className={`fa-toast ${n.kind}`} role={n.kind === 'error' ? 'alert' : 'status'}>
          <span>{n.text}</span>
          {n.detail && <NoteMark kind="info" inline testId="notice-detail" text={n.detail} />}
          {n.action && <button type="button" onClick={() => { n.action!.run(); dismiss(n.id); }}>{n.action.label}</button>}
          {n.action2 && <button type="button" onClick={() => { n.action2!.run(); dismiss(n.id); }}>{n.action2.label}</button>}
          <button type="button" aria-label={t('notice.dismiss')} onClick={() => dismiss(n.id)}>×</button>
        </div>
      ))}
    </div>
  );
}
