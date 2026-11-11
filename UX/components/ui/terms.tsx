import type { ReactNode } from 'react';
import { hasKey, t, type I18nKey } from '../../i18n';
import { AXIS_PATHS } from '../detail/tabs/axisIcons';
import { Tooltip } from './Tooltip';
import './terms.css';

/**
 * Nguồn dùng chung cho thuật ngữ typo: tên tiếng Anh + gloss (i18n `term.<key>.gloss`) + hình SVG nhỏ.
 * Mọi màn (Compare, Giải phẫu, Pair, bộ lọc, chip phân loại…) chỉ gọi <TermTip term="…">, không viết chữ giải thích riêng.
 * Hình vẽ bằng currentColor nên đúng ở cả light và dark; thuật ngữ chưa có hình vẫn hiện gloss.
 */
import { TERM_NAMES, classTermKey, subclassTermKey, termKey, termName } from './termNames';
export { TERM_NAMES, classTermKey, subclassTermKey, termKey, termName };
export const termGloss = (key: string): string | undefined => { const k = `term.${termKey(key)}.gloss`; return hasKey(k) ? t(k as I18nKey) : undefined; };
export const hasTerm = (key: string): boolean => termGloss(key) !== undefined;

// Hình minh hoạ 96×56: baseline y=44. nét 1.8; đường dẫn đứt = đường tham chiếu.
const G = { stroke: 'currentColor', fill: 'none', strokeWidth: 1.8, strokeLinecap: 'round', strokeLinejoin: 'round' } as const;
const guide = (y: number, dash = true) => <path d={`M6 ${y}H90`} strokeDasharray={dash ? '3 3' : undefined} opacity={dash ? 0.55 : 0.9} strokeWidth={dash ? 1.2 : 1.6} />;
const dim = (x: number, y1: number, y2: number) => <path d={`M${x} ${y1}V${y2}M${x - 3} ${y1 + 4}L${x} ${y1}L${x + 3} ${y1 + 4}M${x - 3} ${y2 - 4}L${x} ${y2}L${x + 3} ${y2 - 4}`} className="acc" />;
const ILLU: Record<string, ReactNode> = {
  x_height: <>{guide(44, false)}{guide(26)}<path d="M26 26L46 44M46 26L26 44" />{dim(68, 26, 44)}</>,
  cap_height: <>{guide(44, false)}{guide(10)}<path d="M26 44V10M46 44V10M26 27H46" />{dim(68, 10, 44)}</>,
  ascender: <>{guide(44, false)}{guide(26)}{guide(8)}<path d="M28 44V8M28 28c4-5 16-5 16 5v11" />{dim(68, 8, 26)}</>,
  descender: <>{guide(20, false)}{guide(46)}<path d="M30 12V46M30 24c14-7 20 13 0 15" />{dim(68, 20, 46)}</>,
  weight_class: <><path d="M6 44H90" opacity=".9" strokeWidth="1.6" /><path d="M22 10V44" strokeWidth="2" /><path d="M48 10V44" strokeWidth="6" /><path d="M74 10V44" strokeWidth="11" /></>,
  width_class: <><path d="M6 44H90" opacity=".9" strokeWidth="1.6" /><ellipse cx="22" cy="27" rx="9" ry="17" /><ellipse cx="58" cy="27" rx="17" ry="17" /><path d="M38 50H78" className="acc" /></>,
  stem: <><path d="M6 44H90" opacity=".9" strokeWidth="1.6" /><rect x="40" y="8" width="12" height="36" rx="1" className="fill" /><path d="M40 51H52M40 48v6M52 48v6" className="acc" /></>,
  contrast: <><ellipse cx="48" cy="26" rx="22" ry="19" className="fill" /><ellipse cx="48" cy="26" rx="12" ry="14" className="hole" /><path d="M26 26H36M48 7V12" className="acc" /></>,
  contrast_h: <><path d="M30 8V44M62 8V44" strokeWidth="6" /><path d="M30 26H62" strokeWidth="1.5" /><path d="M24 50H36M56 50H68" className="acc" /></>,
  stress: <><g transform="rotate(-22 48 26)"><ellipse cx="48" cy="26" rx="22" ry="19" className="fill" /><ellipse cx="48" cy="26" rx="12" ry="14" className="hole" /></g><path d="M30 6L66 46" className="acc" strokeDasharray="3 3" /></>,
  slant: <><path d="M6 44H90" opacity=".9" strokeWidth="1.6" /><path d="M42 44L54 8" strokeWidth="5" /><path d="M42 44V8" strokeDasharray="3 3" opacity=".55" strokeWidth="1.2" /><path d="M42 30A14 14 0 0 1 47 31" className="acc" /></>,
  aperture: <><path d="M60 18a19 19 0 1 0 0 16" strokeWidth="5" /><path d="M64 18V34M62 22L64 18L66 22M62 30L64 34L66 30" className="acc" /></>,
  aperture_e: <><path d="M62 34a19 19 0 1 0-2 12M30 29H64V24" strokeWidth="4" /><path d="M68 34V46" className="acc" /></>,
  serif_foot: <><path d="M6 44H90" opacity=".9" strokeWidth="1.6" /><path d="M48 8V38M36 44H60M40 38H56" strokeWidth="3.2" /><path d="M36 50H60" className="acc" /></>,
  overlay: <>{guide(44, false)}<path d="M30 44V10M52 44V10M30 27H52" opacity=".35" strokeWidth="3" /><path d="M36 44V10M58 44V10M36 28H58" className="acc" strokeWidth="3" /></>,
  vi_marks: <>{guide(44, false)}{guide(28)}<path d="M30 36a9 8 0 1 0 18 0a9 8 0 1 0-18 0M48 28V44" /><path d="M31 22L39 14L47 22M45 10L51 5" className="acc" /></>,
  activate: <><rect x="8" y="10" width="30" height="36" rx="3" strokeDasharray="3 3" opacity=".55" strokeWidth="1.2" /><path d="M16 36V20h8M16 28h6" /><path d="M44 28H62M56 22L62 28L56 34" className="acc" /><rect x="66" y="10" width="24" height="36" rx="3" /><path d="M71 18H85M71 24H85M71 30H80" opacity=".6" strokeWidth="1.2" /></>,
  'class.serif': <>{guide(44, false)}<path d="M48 10V40M36 10H60M36 44H60" strokeWidth="2.2" /></>,
  'class.sans': <>{guide(44, false)}<path d="M48 10V44" strokeWidth="6" /></>,
  'class.slab': <>{guide(44, false)}<path d="M48 12V40" strokeWidth="6" /><path d="M34 10H62M34 44H62" strokeWidth="6" className="acc" /></>,
  'class.handwriting': <>{guide(44, false)}<path d="M20 44C18 30 22 14 28 12C34 22 32 36 36 44M44 30C52 22 62 24 60 34C58 42 48 44 48 36M68 44C66 32 72 22 78 20" strokeWidth="2.6" /></>,
  'class.display': <>{guide(44, false)}<path d="M30 26a18 18 0 0 1 14-17M52 9a18 18 0 0 1 14 17M66 30a18 18 0 0 1-14 14M44 44a18 18 0 0 1-14-14" strokeWidth="7" /><path d="M44 5V12M52 5V12M44 41V48M52 41V48" className="acc" /></>,
  'sub.flared': <>{guide(44, false)}<path className="fill" d="M42 8H54L51.5 26L54 44H42L44.5 26Z" /></>,
  'sub.squarish': <><rect x="30" y="10" width="36" height="34" rx="10" strokeWidth="5" /><path d="M30 6H66" className="acc" /></>,
  'sub.venetian_oldstyle': <>{guide(44, false)}<path d="M66 31a17 14 0 1 0-4 9" strokeWidth="3.2" /><path d="M32 31L64 23" className="acc" strokeWidth="2.6" /></>,
  'sub.garalde': <>{guide(44, false)}<path d="M66 33a17 14 0 1 0-4 9" strokeWidth="3.2" /><path d="M32 31H65" className="acc" strokeWidth="2.6" /></>,
  'sub.monoline_script': <>{guide(44, false)}<path d="M12 40C24 6 32 6 32 24C32 42 42 42 50 28C58 14 64 44 84 22" strokeWidth="3" /><path d="M8 52H88" className="acc" strokeDasharray="2 4" /></>,
  'sub.coding': <><rect x="34" y="8" width="28" height="36" rx="12" strokeWidth="3.2" /><path d="M40 38L56 14" className="acc" strokeWidth="2.6" /><path d="M10 50H86" strokeDasharray="2 4" opacity=".55" strokeWidth="1.2" /></>,
  hybrid: <><circle cx="38" cy="26" r="16" /><circle cx="58" cy="26" r="16" className="acc" /></>,
  baseline: <>{guide(44, false)}<path d="M30 44V16M30 26c4-4 14-4 14 5v13" /></>,
};
/** Thuật ngữ đã có hình riêng, hoặc dùng lại icon trục (24×24) phóng to. */
function TermIllu({ term }: { term: string }): React.JSX.Element | null {
  const k = termKey(term);
  if (ILLU[k]) return <svg className="fa-term-art" viewBox="0 0 96 56" width="96" height="56" aria-hidden {...G}>{ILLU[k]}</svg>;
  if (AXIS_PATHS[k]) return <svg className="fa-term-art small" viewBox="0 0 24 24" width="48" height="48" aria-hidden {...G} strokeWidth={1.4}>{AXIS_PATHS[k]}</svg>;
  return null;
}

/** Thẻ giải thích: tên EN, gloss, hình (nếu có), và một dòng phụ tuỳ ý (vd quan hệ cặp). */
export function TermCard({ term, extra }: { term: string; extra?: ReactNode }): React.JSX.Element | null {
  const gloss = termGloss(term);
  if (!gloss) return null;
  const key = termKey(term);
  return (
    <span className="fa-term" data-term-card={key}>
      <TermIllu term={term} />
      <b>{TERM_NAMES[key] ?? key}</b>
      <span className="fa-term-gloss">{gloss}</span>
      {extra ? <span className="fa-term-extra">{extra}</span> : null}
    </span>
  );
}

/** Bọc một nhãn thuật ngữ: hover/focus hiện thẻ giải thích. Không có gloss cho thuật ngữ thì giữ nguyên phần tử, không có tooltip. */
export function TermTip({ term, extra, children }: { term: string; extra?: ReactNode; children: React.ReactElement }): React.JSX.Element {
  const key = termKey(term);
  if (!hasTerm(key)) return children;
  return <Tooltip label={<TermCard term={key} extra={extra} />}>{children}</Tooltip>;
}

/** Như TermTip nhưng khi thuật ngữ chưa có gloss thì vẫn hiện dòng phụ (vd quan hệ cặp) bằng tooltip thường. */
export function TermOrTip({ term, extra, children }: { term: string; extra: ReactNode; children: React.ReactElement }): React.JSX.Element {
  return hasTerm(term) ? <TermTip term={term} extra={extra}>{children}</TermTip> : <Tooltip label={extra}>{children}</Tooltip>;
}
