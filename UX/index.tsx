import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';
import { setApi } from './api';
import { createApi } from './api/select';
import { applyDevBoot, applyDevPreset, devInitial } from './store/devPreset';
import { LocaleBoundary } from './i18n/LocaleBoundary';
import { chooseLocale, initLocale, hasChosenThisSession } from './i18n';

// Trong cửa sổ Tauri: lệnh thật (UX/bindings). Ngoài Tauri hoặc VITE_FA_API=mock: mock, kèm preset trạng thái cho dev/QA.
const dev = import.meta.env.DEV;
const { api } = createApi(dev ? { initial: devInitial(location.hash), tickMs: 1500, firstRun: /[#&]first=1(&|$)/.test(location.hash) } : {}); // #first=1: mở như lần đầu (onboarding) — chỉ dev/QA
setApi(api);
// QA native chọn ngôn ngữ xác định (không phụ thuộc ngôn ngữ của máy chạy QA).
(window as unknown as { __faLocale?: typeof chooseLocale }).__faLocale = chooseLocale;

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error("Could not find root element to mount to");
}

const root = ReactDOM.createRoot(rootElement);
const render = () => root.render(
  <React.StrictMode>
    <LocaleBoundary>
      {() => (
        <App />
      )}
    </LocaleBoundary>
  </React.StrictMode>
);
// Ngôn ngữ giao diện quyết định trước lần vẽ đầu: lựa chọn đã lưu → setting backend → ngôn ngữ ưu tiên của hệ điều hành.
// Chờ tối đa LANG_WAIT_MS: backend chậm thì vẽ theo ngôn ngữ hệ thống, BootGate đọc lại `uiLang` khi backend sẵn sàng.
const LANG_WAIT_MS = 300;
const langReady = Promise.race([
  api.getSettings().then(s => { if (!hasChosenThisSession()) initLocale(s.uiLang); }, () => { if (!hasChosenThisSession()) initLocale(null); }),
  new Promise<void>(res => setTimeout(() => { if (!hasChosenThisSession()) initLocale(null); res(); }, LANG_WAIT_MS)),
]);
if (dev) {
  void applyDevBoot(api, location.hash).catch(e => console.warn('dev boot', e)).then(() => langReady).finally(() => {
    render();
    setTimeout(() => void applyDevPreset(api, location.hash), 400); // sau hydrate()
  });
} else void langReady.finally(render);
