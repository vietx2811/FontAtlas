import { getLocale, type Locale } from './locale';

/** Chữ mẫu mặc định: trung tính (người dùng là designer / dàn trang, không riêng F&B), có dấu chồng tiếng Việt để thấy ngay lỗi dấu. */
export const SAMPLE_DEFAULT: Record<Locale, string> = { vi: 'Chữ Việt đẹp — Ấn tượng', en: 'Typeface · Chữ Việt' };
/** Mặc định cũ (mang chất F&B); backend vẫn có thể trả nó khi người dùng chưa từng đổi, nên coi như "chưa đổi". */
export const SAMPLE_LEGACY = 'Phở bò Hà Nội';

export const defaultSample = (loc: Locale = getLocale()): string => SAMPLE_DEFAULT[loc];
/** Chữ mẫu người dùng CHƯA đổi (mặc định của bất kỳ ngôn ngữ nào, hoặc mặc định cũ): đổi theo ngôn ngữ; chữ do người dùng gõ thì không bao giờ bị ghi đè. */
export const isDefaultSample = (s: string): boolean => s === SAMPLE_LEGACY || s === SAMPLE_DEFAULT.vi || s === SAMPLE_DEFAULT.en;
