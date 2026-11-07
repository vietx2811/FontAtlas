import { useEffect, useRef } from 'react';
import { useUiStore } from '../../store/ui';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { Settings } from './Settings';

type Props = { api: Api; onRerunOnboarding(): void };

/**
 * Cài đặt dạng lớp phủ trong vùng nội dung: nền mờ, bấm ngoài hoặc Esc để đóng.
 * Sidebar vẫn bấm được (đóng lớp phủ rồi chuyển mục) nên đây không phải modal tuyệt đối:
 * phần nội dung bên dưới bị `inert`, tiêu điểm vào sheet khi mở và trả về nơi cũ khi đóng.
 */
function Sheet({ api, onRerunOnboarding }: Props): React.JSX.Element {
  const scrim = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const close = () => useUiStore.getState().setSettingsOpen(false);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    const below = [...(scrim.current?.parentElement?.children ?? [])].filter(c => c !== scrim.current) as HTMLElement[];
    below.forEach(c => { c.inert = true; });
    box.current?.focus();
    return () => {
      below.forEach(c => { c.inert = false; });
      if (before && before.isConnected) before.focus();
    };
  }, []);
  return (
    <div ref={scrim} className="fa-set-scrim" data-testid="settings-scrim" onMouseDown={e => { if (e.target === e.currentTarget) close(); }}>
      <div ref={box} role="dialog" tabIndex={-1} aria-label={t('settings.title')} className="fa-set-sheet">
        <Settings api={api} onClose={close} onRerunOnboarding={onRerunOnboarding} />
      </div>
    </div>
  );
}

export function SettingsOverlay(props: Props): React.JSX.Element | null {
  return useUiStore(s => s.settingsOpen) ? <Sheet {...props} /> : null;
}
