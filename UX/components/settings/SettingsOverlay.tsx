import { useUiStore } from '../../store/ui';
import type { Api } from '../../api';
import { Settings } from './Settings';

type Props = { api: Api; onRerunOnboarding(): void };

/** Cài đặt dạng lớp phủ trong vùng nội dung: nền mờ, bấm ngoài hoặc Esc để đóng; sidebar vẫn bấm được. */
export function SettingsOverlay({ api, onRerunOnboarding }: Props): React.JSX.Element | null {
  const open = useUiStore(s => s.settingsOpen);
  if (!open) return null;
  const close = () => useUiStore.getState().setSettingsOpen(false);
  return (
    <div className="fa-set-scrim" data-testid="settings-scrim" onMouseDown={e => { if (e.target === e.currentTarget) close(); }}>
      <div role="dialog" aria-modal="true" aria-label="Cài đặt" className="fa-set-sheet">
        <Settings api={api} onClose={close} onRerunOnboarding={onRerunOnboarding} />
      </div>
    </div>
  );
}
