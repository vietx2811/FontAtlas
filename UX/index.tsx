import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';
import { setApi } from './api';
import { createApi } from './api/select';
import { applyDevBoot, applyDevPreset, devInitial } from './store/devPreset';
import { StatusProvider } from './context/StatusContext';

// Trong cửa sổ Tauri: lệnh thật (UX/bindings). Ngoài Tauri hoặc VITE_FA_API=mock: mock, kèm preset trạng thái cho dev/QA.
const dev = import.meta.env.DEV;
const { api } = createApi(dev ? { initial: devInitial(location.hash), tickMs: 1500 } : {});
setApi(api);

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error("Could not find root element to mount to");
}

const root = ReactDOM.createRoot(rootElement);
const render = () => root.render(
  <React.StrictMode>
    <StatusProvider>
      <App />
    </StatusProvider>
  </React.StrictMode>
);
if (dev) {
  void applyDevBoot(api, location.hash).catch(e => console.warn('dev boot', e)).finally(() => {
    render();
    setTimeout(() => void applyDevPreset(api, location.hash), 400); // sau hydrate()
  });
} else render();
