/**
 * Ghi công bên thứ ba. Giấy phép đã đối chiếu với manifest của từng crate/package (Cargo.toml, package.json)
 * và model card; cập nhật khi đổi phiên bản hoặc thêm phụ thuộc. Tên riêng và giấy phép là dữ kiện, không dịch;
 * mô tả (`what`) là khoá i18n.
 */
import type { I18nKey } from '../../i18n';
export type CreditGroup = 'data' | 'model' | 'engine' | 'ui';
export type Credit = { id: string; name: string; by: string; licence: string; what: I18nKey; group: CreditGroup; /** Văn bản giấy phép đầy đủ có trong bản cài (Resources/). */ fullText?: string };

export const CREDITS: readonly Credit[] = [
  { id: 'hyperglot', name: 'Hyperglot', by: 'Rosetta Type/Research & contributors', licence: 'Apache-2.0', what: 'credits.what.hyperglot', group: 'data', fullText: 'LICENSE-hyperglot.txt' },
  { id: 'siglip2', name: 'SigLIP 2 (siglip2-base-patch16-224)', by: 'Google', licence: 'Apache-2.0', what: 'credits.what.siglip2', group: 'model' },
  { id: 'candle', name: 'candle', by: 'Hugging Face', licence: 'MIT OR Apache-2.0', what: 'credits.what.candle', group: 'model' },
  { id: 'tauri', name: 'Tauri', by: 'Tauri Programme (CrabNebula)', licence: 'Apache-2.0 OR MIT', what: 'credits.what.tauri', group: 'engine' },
  { id: 'fontations', name: 'Fontations (read-fonts, skrifa) · kurbo', by: 'Google Fonts · Linebender', licence: 'MIT OR Apache-2.0', what: 'credits.what.fontations', group: 'engine' },
  { id: 'wuff', name: 'wuff', by: 'wuff contributors', licence: 'MIT', what: 'credits.what.wuff', group: 'engine' },
  { id: 'tiny-skia', name: 'tiny-skia', by: 'Yevhenii Reizner & contributors', licence: 'BSD-3-Clause', what: 'credits.what.tinySkia', group: 'engine' },
  { id: 'sqlite', name: 'SQLite · rusqlite', by: 'SQLite Consortium · rusqlite contributors', licence: 'Public domain · MIT', what: 'credits.what.sqlite', group: 'engine' },
  { id: 'blake3', name: 'BLAKE3 · png · notify', by: 'BLAKE3 team · image-rs · notify-rs', licence: 'CC0-1.0 OR Apache-2.0 · MIT OR Apache-2.0 · CC0-1.0', what: 'credits.what.misc', group: 'engine' },
  { id: 'react', name: 'React · react-window · zustand', by: 'Meta · Brian Vaughn · Poimandres', licence: 'MIT', what: 'credits.what.react', group: 'ui' },
  { id: 'libs', name: 'framer-motion · recharts · clsx · tailwind-merge', by: 'Framer · Recharts Group · Luke Edwards · dcastil', licence: 'MIT', what: 'credits.what.uiLibs', group: 'ui' },
  { id: 'lucide', name: 'Lucide', by: 'Lucide Contributors', licence: 'ISC', what: 'credits.what.lucide', group: 'ui' },
];
export const CREDIT_GROUPS: readonly CreditGroup[] = ['data', 'model', 'engine', 'ui'];
