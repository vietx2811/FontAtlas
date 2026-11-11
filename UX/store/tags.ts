import { create } from 'zustand';

/** Tên thẻ cho chip bộ lọc ("Thẻ: serif"); thanh bên nạp danh sách và ghi vào đây. */
export const useTagsStore = create<{ tags: ReadonlyArray<{ id: number; name: string }>; set(t: ReadonlyArray<{ id: number; name: string }>): void }>()(set => ({ tags: [], set: tags => set({ tags }) }));
