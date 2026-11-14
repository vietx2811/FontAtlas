import { create } from 'zustand';
import type { FaceKeyDto } from '../api';

/** G7b: hộp thoại "Tạo bản sao đổi tên…" đang mở cho font VF nào (null = đóng). Mở từ dấu "!" của ClashNote. */
type State = { target: { face: FaceKeyDto; label: string } | null; open(face: FaceKeyDto, label: string): void; close(): void };
export const useDeriveStore = create<State>()(set => ({ target: null, open: (face, label) => set({ target: { face, label } }), close: () => set({ target: null }) }));
