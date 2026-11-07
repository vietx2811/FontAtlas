import { create } from 'zustand';
import type { Api, SourceDto } from '../api';

type S = { sources: SourceDto[]; key: string | null; load(api: Api, key: string): Promise<void> };
/** Danh sách nguồn dùng chung cho chip/bộ lọc. `key` (vd phiên bản quét) chống tải lặp: cùng key chỉ tải một lần. */
export const useSourcesStore = create<S>((set, get) => ({
  sources: [], key: null,
  load: async (api, key) => {
    if (get().key === key) return;
    set({ key });
    try { set({ sources: await api.listSources() }); } catch { set({ key: null }); }
  },
}));
