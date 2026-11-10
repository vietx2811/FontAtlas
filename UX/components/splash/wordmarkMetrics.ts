// Tạo bởi tools/design/measure_wordmark_fonts.py. Không sửa tay.
export type WordmarkFont = { family: string; weight: number; upm: number; xHeight: number };
/** Newsreader 500, đỉnh phẳng của 'x' = 885/2000 em. Giấy phép: public/fonts/OFL-Newsreader.txt. */
export const SERIF: WordmarkFont = { family: 'Newsreader', weight: 500, upm: 2000, xHeight: 885 };
/** Inter 600, đỉnh phẳng của 'x' = 1118/2048 em. Giấy phép: public/fonts/OFL-Inter.txt. */
export const SANS: WordmarkFont = { family: 'Inter', weight: 600, upm: 2048, xHeight: 1118 };
/** JetBrains Mono 500, đỉnh phẳng của 'x' = 550/1000 em. Giấy phép: public/fonts/OFL-JetBrainsMono.txt. */
export const MONO: WordmarkFont = { family: 'JetBrains Mono', weight: 500, upm: 1000, xHeight: 550 };
