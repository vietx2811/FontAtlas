import { create } from 'zustand';

/** B5: hộp thoại "Mở cùng app…" đang mở cho Case nào (null = đóng). Mở được từ Desk và từ menu Case ở thanh bên. */
type State = { target: { caseId: number; name: string } | null; open(caseId: number, name: string): void; close(): void };
export const useAppPickerStore = create<State>()(set => ({ target: null, open: (caseId, name) => set({ target: { caseId, name } }), close: () => set({ target: null }) }));
