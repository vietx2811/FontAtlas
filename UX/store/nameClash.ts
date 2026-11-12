// G7: font Variable trùng tên PostScript với bản static. Tên hiển thị của bản VF thêm hậu tố "VF"; không sửa file.
import { useMemo } from 'react';
import { create } from 'zustand';
import type { Api } from '../api';
import type { NameClashDto } from '../bindings';

type State = {
  /** vfFaceId → cặp trùng. */
  vf: Record<number, NameClashDto>;
  /** staticFaceId → các VF trùng với nó. */
  st: Record<number, number[]>;
  set(list: ReadonlyArray<NameClashDto>): void;
};

export const useNameClashStore = create<State>()(set => ({
  vf: {}, st: {},
  set: list => {
    const vf: Record<number, NameClashDto> = {};
    const st: Record<number, number[]> = {};
    for (const c of list) {
      vf[c.vfFaceId] = c;
      for (const s of c.staticFaceIds) (st[s] ??= []).push(c.vfFaceId);
    }
    set({ vf, st });
  },
}));

/** Nạp lại danh sách trùng tên; lỗi thì giữ danh sách cũ (không làm vỡ giao diện). */
export async function refreshNameClashes(api: Api): Promise<void> {
  try { useNameClashStore.getState().set(await api.nameClashes()); } catch (e) { console.warn('name_clashes failed', e); }
}

export type NameClashRole = 'vf' | 'static';
/** Vai của một face trong cặp trùng tên, `null` nếu không trùng. */
export function useNameClashRole(faceId: number | undefined): { role: NameClashRole; exact: boolean; names: string[] } | null {
  const vf = useNameClashStore(s => (faceId === undefined ? undefined : s.vf[faceId])); // tham chiếu ổn định
  const isStatic = useNameClashStore(s => faceId !== undefined && !!s.st[faceId]);
  return useMemo(() => (vf ? { role: 'vf' as const, exact: vf.exact, names: vf.psNames } : isStatic ? { role: 'static' as const, exact: true, names: [] } : null), [vf, isStatic]);
}

/** Tên hiển thị: bản VF trùng tên thêm hậu tố " VF" (chỉ hiển thị, không đổi tên thật). */
export const vfDisplayName = (name: string, role: NameClashRole | undefined): string => (role === 'vf' && !/\bVF$/.test(name) ? `${name} VF` : name);
