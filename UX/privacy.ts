// Mở Cài đặt hệ thống (Quyền riêng tư và Bảo mật) để người dùng cấp quyền truy cập thư mục.
export const PRIVACY_URLS = {
  files: 'x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders',
  disk: 'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles',
} as const;

type Opener = (url: string) => Promise<boolean | void>;
// Mặc định: plugin opener của Tauri (chỉ cho phép đúng scheme x-apple.systempreferences, xem capabilities/open-settings.json).
let opener: Opener = async url => {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return false;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:opener|open_url', { url });
  return true;
};
export function setUrlOpener(o: Opener): void { opener = o; }

/** true nếu đã mở; false khi không mở được (giao diện chỉ đường thủ công: Cài đặt hệ thống › Quyền riêng tư › Tệp và thư mục). */
export async function openPrivacySettings(pane: keyof typeof PRIVACY_URLS = 'files'): Promise<boolean> {
  try { return (await opener(PRIVACY_URLS[pane])) !== false; } catch { return false; }
}
