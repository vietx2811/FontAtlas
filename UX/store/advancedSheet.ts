import { create } from 'zustand';

/** Sheet Advanced mở/đóng; ⌥⌘F và nút "Advanced" cùng ghi vào đây. */
export const useAdvancedSheet = create<{ open: boolean; setOpen(o: boolean): void; toggle(): void }>()(set => ({
  open: false, setOpen: open => set({ open }), toggle: () => set(s => ({ open: !s.open })),
}));
