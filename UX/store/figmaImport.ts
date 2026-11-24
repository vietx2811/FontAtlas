import { create } from 'zustand';
import type { FigmaImportDto } from '../bindings';

/** Hộp thoại "Dán từ Figma": `paste` = ô dán; `preview` = đã có danh sách (từ link hoặc từ ô dán) cần khớp với thư viện. */
type State = {
  phase: 'closed' | 'paste' | 'preview';
  imp: FigmaImportDto | null;
  openPaste(): void;
  show(imp: FigmaImportDto): void;
  close(): void;
};

export const useFigmaImport = create<State>(set => ({
  phase: 'closed', imp: null,
  openPaste: () => set({ phase: 'paste', imp: null }),
  show: imp => set({ phase: 'preview', imp }),
  close: () => set({ phase: 'closed', imp: null }),
}));
