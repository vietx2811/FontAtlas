// Nhận ra ý đồ từ tên combo (G6): bảng tĩnh vi + en, chạy ở client, không gọi mạng, không LLM (spec G5/G6 §3.2, Phụ lục A).
// Mỗi ý đồ chỉ quyết định: bộ vai, các phong cách pair ưu tiên (id của G4). Backend không đoán ý đồ từ tên.
import { t } from '../../i18n';

export type IntentId = 'ui' | 'editorial' | 'brand' | 'packaging' | 'poster';
export type Intent = {
  id: IntentId;
  /** Tên phụ nhận được (không phân biệt hoa thường, bỏ dấu). Cụm nhiều từ khớp như cụm. */
  aliases: readonly string[];
  /** Khoá vai preset (G3). */ roles: readonly string[]; styles: readonly string[];
};

export const INTENTS: readonly Intent[] = [
  { id: 'ui', aliases: ['ui', 'app', 'ung dung', 'giao dien', 'web', 'website', 'dashboard', 'saas', 'tech startup', 'startup', 'cong nghe', 'product'], roles: ['heading', 'body', 'label'], styles: ['superfamily', 'same_structure'] },
  { id: 'editorial', aliases: ['editorial', 'tap chi', 'magazine', 'bao', 'article', 'bai dai', 'sach', 'book', 'longform', 'newsletter'], roles: ['display', 'heading', 'body', 'caption', 'quote'], styles: ['contrast', 'display_vs_neutral'] },
  { id: 'brand', aliases: ['brand', 'thuong hieu', 'identity', 'nhan dien', 'logo', 'brand book', 'branding', 'guideline'], roles: ['display', 'heading', 'body'], styles: ['superfamily', 'contrast'] },
  { id: 'packaging', aliases: ['packaging', 'menu', 'bao bi', 'nhan', 'label', 'thuc don', 'nha hang', 'restaurant', 'cafe', 'f&b', 'fnb', 'bao bi san pham'], roles: ['display', 'heading', 'caption', 'numerals'], styles: ['display_vs_neutral', 'contrast'] },
  { id: 'poster', aliases: ['poster', 'campaign', 'ap phich', 'flyer', 'banner', 'quang cao', 'ads', 'chien dich', 'key visual', 'kv'], roles: ['display', 'heading', 'body'], styles: ['display_vs_neutral', 'harmony'] },
];

/** Bỏ dấu, hạ chữ thường, gộp khoảng trắng; "đ" → "d". */
export const normalizeName = (s: string): string => s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/đ/gi, 'd').toLowerCase().replace(/[^a-z0-9& ]+/g, ' ').replace(/\s+/g, ' ').trim();

/**
 * Khớp tên với ý đồ: cụm tên phụ khớp theo từ (hoặc tiền tố của từ cuối đang gõ dở, từ 3 chữ).
 * Nhiều ý đồ cùng khớp → ý đồ có tên phụ DÀI nhất (cụ thể nhất) thắng; hoà thì ý đồ đứng trước trong bảng.
 */
export function matchIntent(raw: string): Intent | null {
  const name = normalizeName(raw);
  if (!name) return null;
  const words = name.split(' ');
  let best: { intent: Intent; len: number } | null = null;
  for (const intent of INTENTS) {
    for (const a of intent.aliases) {
      const aw = a.split(' ');
      for (let i = 0; i + aw.length <= words.length; i++) {
        const seg = words.slice(i, i + aw.length);
        const last = i + aw.length === words.length;
        const ok = seg.every((w, k) => w === aw[k] || (last && k === seg.length - 1 && w.length >= 3 && aw[k].startsWith(w)));
        if (ok && (!best || a.length > best.len)) best = { intent, len: a.length };
      }
    }
  }
  return best?.intent ?? null;
}
/** Tên hiển thị theo locale (khoá `intent.<id>`). */
export const intentName = (i: Intent): string => t(`intent.${i.id}` as 'intent.ui');
