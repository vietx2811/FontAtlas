import type { Api } from './index';
import { createMockApi, type MockOptions } from './mock';
import { createTauriApi } from './tauri';

export type ApiKind = 'tauri' | 'mock';

/** VITE_FA_API=mock|tauri ép chọn; không ép thì: trong cửa sổ Tauri dùng lệnh thật, ngoài Tauri (trình duyệt, Vitest) dùng mock. */
export function chooseApiKind(env: { VITE_FA_API?: string }, inTauri: boolean): ApiKind {
  if (env.VITE_FA_API === 'tauri') return 'tauri';
  if (env.VITE_FA_API === 'mock') return 'mock';
  return inTauri ? 'tauri' : 'mock';
}

export function createApi(mockOptions: MockOptions = {}): { api: Api; kind: ApiKind } {
  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const kind = chooseApiKind(import.meta.env as { VITE_FA_API?: string }, inTauri);
  return { api: kind === 'tauri' ? createTauriApi() : createMockApi(mockOptions), kind };
}
