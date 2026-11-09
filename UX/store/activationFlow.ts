import { create } from 'zustand';
import type { FaceKeyDto } from '../api';
import type { ConflictItem } from '../components/activation/conflictModel';

export type PendingActivation =
  | { type: 'conflict'; total: number; conflictFaces: FaceKeyDto[]; items: ConflictItem[] }
  | { type: 'held'; faces: FaceKeyDto[]; collections: number };

/**
 * Hộp thoại kích hoạt đang chờ người dùng, MỘT hàng đợi cho cả app (không theo từng nơi gọi): nhiều nơi cùng bật font
 * (side peek, thanh chọn, phím tắt) không dựng chồng hai hộp thoại, và hộp đang mở không mất khi nơi gọi bị gỡ khỏi cây.
 * Hộp đầu hàng đợi hiện; xong thì hộp kế tiếp hiện.
 */
export const useActivationQueue = create<{ queue: PendingActivation[]; enqueue(p: PendingActivation): void; shift(): void; clear(): void }>(set => ({
  queue: [],
  enqueue: p => set(s => ({ queue: [...s.queue, p] })),
  shift: () => set(s => ({ queue: s.queue.slice(1) })),
  clear: () => set({ queue: [] }),
}));
