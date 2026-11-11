import { create } from 'zustand';

type Tag = { id: number; name: string };
/** Tên thẻ cho chip bộ lọc ("Thẻ: serif"); thanh bên nạp danh sách và ghi vào đây. `loaded`: đã nạp ít nhất một lần (chưa nạp thì chip không hiện "#12"). */
export const useTagsStore = create<{ tags: ReadonlyArray<Tag>; loaded: boolean; set(t: ReadonlyArray<Tag>): void }>()(set => ({ tags: [], loaded: false, set: tags => set({ tags, loaded: true }) }));
