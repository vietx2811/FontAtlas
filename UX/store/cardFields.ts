import { create } from 'zustand';
import type { Api } from '../api';

/** Thông tin hiện trên thẻ font (spec library-view-options A3). Lát L4: các trường đã có dữ liệu trong `FaceRow`. */
export const CARD_FIELD_ORDER = ['format', 'lang', 'license', 'variable'] as const;
export type CardField = (typeof CARD_FIELD_ORDER)[number];
export const DEFAULT_CARD_FIELDS: readonly CardField[] = ['format', 'lang'];
export const CARD_FIELDS_KEY = 'ui.card_fields';
const PERSIST_MS = 400;
const known = (x: unknown): x is CardField => typeof x === 'string' && (CARD_FIELD_ORDER as readonly string[]).includes(x);
/** Chuẩn hoá theo thứ tự cố định, bỏ khoá lạ và trùng. */
const normalize = (xs: readonly unknown[]): CardField[] => CARD_FIELD_ORDER.filter(k => xs.includes(k));

/** Dữ liệu hỏng, phiên bản lạ hoặc thiếu → mặc định (không báo lỗi). */
export function parseCardFields(raw: string | null | undefined): CardField[] {
  if (!raw) return [...DEFAULT_CARD_FIELDS];
  try {
    const o = JSON.parse(raw) as { v?: unknown; fields?: unknown };
    if (o?.v !== 1 || !Array.isArray(o.fields)) return [...DEFAULT_CARD_FIELDS];
    return normalize(o.fields.filter(known));
  } catch { return [...DEFAULT_CARD_FIELDS]; }
}
export const serializeCardFields = (f: readonly CardField[]): string => JSON.stringify({ v: 1, fields: normalize(f) });

type State = { fields: CardField[]; set(f: readonly CardField[]): void; toggle(k: CardField): void; reset(): void };
export const useCardFields = create<State>()((set, get) => ({
  fields: [...DEFAULT_CARD_FIELDS],
  set: f => set({ fields: normalize(f) }),
  toggle: k => set({ fields: normalize(get().fields.includes(k) ? get().fields.filter(x => x !== k) : [...get().fields, k]) }),
  reset: () => set({ fields: [...DEFAULT_CARD_FIELDS] }),
}));

/** Đọc `ui.card_fields` rồi ghi ngược (debounce) khi đổi. Trả hàm dừng. */
export async function hydrateCardFields(api: Api): Promise<() => void> {
  try { useCardFields.getState().set(parseCardFields(await api.getSetting(CARD_FIELDS_KEY))); } catch { useCardFields.getState().reset(); }
  let written = serializeCardFields(useCardFields.getState().fields); let timer: ReturnType<typeof setTimeout> | undefined;
  const unsub = useCardFields.subscribe(s => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const json = serializeCardFields(s.fields);
      if (json === written) return;
      written = json;
      api.setSetting(CARD_FIELDS_KEY, json).catch(e => console.warn('set_setting failed', e));
    }, PERSIST_MS);
  });
  return () => { unsub(); clearTimeout(timer); };
}
