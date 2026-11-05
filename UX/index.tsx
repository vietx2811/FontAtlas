import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';
import { setApi } from './api';
import { createMockApi } from './api/mock';
import { applyDevPreset } from './store/devPreset';
import { StatusProvider } from './context/StatusContext';

// Đến khi 6a có UX/bindings thật, Api thật thay chỗ này.
const api = createMockApi();
setApi(api);
if (import.meta.env.DEV) setTimeout(() => void applyDevPreset(api, location.hash), 400); // sau hydrate()

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error("Could not find root element to mount to");
}

const root = ReactDOM.createRoot(rootElement);
root.render(
  <React.StrictMode>
    <StatusProvider>
      <App />
    </StatusProvider>
  </React.StrictMode>
);
