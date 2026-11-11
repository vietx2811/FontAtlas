// Splash khi mở app (cũng là phần mở đầu của onboarding lần đầu — một component duy nhất).
// "FontAtlas" hiện ra bằng cách đảo font NGẪU NHIÊN (từ thư viện của người dùng, lọc an toàn; thiếu thì dùng font đóng gói), nhịp chậm dần,
// từng ký tự khoá lệch nhịp; frame cuối: "Font" = Newsreader, "Atlas" = Inter, CÙNG x-height, chung baseline, không khoảng trắng.
// Bấm/phím bất kỳ = bỏ qua. prefers-reduced-motion = hiện thẳng frame cuối. Không chặn app: chỉ là lớp phủ, app dựng song song bên dưới.
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { sharedFontLoader } from '../../features/library-grid/sharedLoader';
import { setSplashOpen } from './splashState';
import { usePrefersReducedMotion } from '../onboarding/blueprint';
import { BUNDLED_POOL, MAX_USER_FONTS, REDUCED_HOLD_MS, SANS_PART, SERIF_PART, SPLASH_MS, TICKS, WORD, finalFont, pickFonts, sizeForXHeight, stepSlots, tickDelays, xRatio, type PoolFont, type Slot } from './splashModel';
import { SANS } from './wordmarkMetrics';
import './splash.css';

type Props = {
  /** Có api thì lấy font ngẫu nhiên từ thư viện; không (hoặc thư viện trống) thì chỉ font đóng gói. */
  api?: Pick<Api, 'queryFaces' | 'startupStatus'> & Parameters<typeof sharedFontLoader>[0];
  onDone(): void; durationMs?: number; rand?: () => number;
  /** Chỉ QA/ảnh chụp: không tự kết thúc. */ hold?: boolean;
};

/** x-height/em đo bằng canvas với font đã nạp; null khi không đo được (jsdom) hoặc kết quả vô lý. */
export function measureXRatio(family: string): number | null {
  try {
    const ctx = document.createElement('canvas').getContext('2d');
    if (!ctx) return null;
    ctx.font = `200px "${family}"`;
    const m = ctx.measureText('x');
    const r = m.actualBoundingBoxAscent / 200;
    return r > 0.25 && r < 0.9 ? r : null;
  } catch { return null; }
}

/** keyup của phím vừa bỏ qua splash cũng không được tới phần dưới. */
const swallowUp = (e: KeyboardEvent) => { if (!(e.metaKey || e.ctrlKey || e.altKey)) { e.stopPropagation(); e.stopImmediatePropagation(); } };

const initialSlots = (final: boolean, pool: readonly PoolFont[], rand: () => number): Slot[] =>
  Array.from({ length: WORD.length }, (_, i) => final ? { font: finalFont(i), locked: true } : { font: pool[Math.floor(rand() * pool.length)], locked: false });

export function Splash({ api, onDone, durationMs = SPLASH_MS, rand = Math.random, hold = false }: Props): React.JSX.Element {
  const reduced = usePrefersReducedMotion();
  const pool = useRef<PoolFont[]>([...BUNDLED_POOL]);
  const [slots, setSlots] = useState<Slot[]>(() => initialSlots(reduced, pool.current, rand));
  const [final, setFinal] = useState(reduced);
  const [leaving, setLeaving] = useState(false);
  const done = useRef(onDone); done.current = onDone;
  const stage = useRef<HTMLDivElement>(null); const mark = useRef<HTMLSpanElement>(null);
  const [guides, setGuides] = useState<{ base: number; x: number } | null>(null);

  // Báo "splash đang mở" để Onboarding/phím tắt không nhận phím bên dưới.
  useEffect(() => { setSplashOpen(true); return () => setSplashOpen(false); }, []);
  const left = useRef(false); const fade = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(fade.current), []);
  const finish = useCallback(() => {
    if (left.current) return;
    left.current = true; setLeaving(true);
    fade.current = setTimeout(() => done.current(), 220); // mờ dần rồi mới gỡ lớp phủ
  }, []);

  // Font người dùng: lọc (có file thật, không ở ổ cloud, đủ glyph = ngôn ngữ eng đủ, ≤ 14), nạp qua bộ nạp preview chung, thêm vào pool khi sẵn sàng.
  useEffect(() => {
    if (!api || reduced) return;
    let dead = false; const held: Array<{ hashHex: string; faceIndex: number; fileSize: number }> = [];
    const loader = sharedFontLoader(api).loader;
    (async () => {
      // Chỉ hỏi thư viện khi backend đã sẵn sàng (splash chạy song song với BootGate): chờ tối đa ~1,5 s, không thì chỉ font đóng gói.
      for (let i = 0; i < 4; i++) {
        if ((await api.startupStatus()).phase === 'ready') break;
        if (i === 3) return;
        await new Promise(r => setTimeout(r, 450));
        if (dead) return;
      }
      if (dead) return;
      const langs = [{ code: 'eng', level: 'full' as const }];
      const first = await api.queryFaces({ langs, limit: 1 });
      if (dead || first.total === 0) return;
      const page = await api.queryFaces({ langs, offset: Math.floor(rand() * Math.max(1, first.total - 60)), limit: 60 });
      if (dead) return;
      for (const r of pickFonts(page.items, rand, MAX_USER_FONTS)) {
        const key = { hashHex: r.hashHex, faceIndex: r.faceIndex, fileSize: r.fileSize };
        held.push(key);
        loader.acquire(key, { soft: true }).then(fam => {
          if (dead || !fam) return;
          pool.current.push({ family: fam, weight: 400, ratio: measureXRatio(fam) ?? xRatio(SANS), user: true });
        }, () => { /* bỏ font này */ });
      }
    })().catch(() => { /* thư viện chưa sẵn sàng: chỉ font đóng gói */ });
    return () => { dead = true; for (const k of held) loader.release(k, { soft: true }); };
  }, [api, reduced, rand]);

  // Nhịp đảo font: chậm dần, rồi frame cuối.
  useEffect(() => {
    if (reduced) { const h = setTimeout(finish, hold ? 1e9 : REDUCED_HOLD_MS); return () => clearTimeout(h); }
    const delays = tickDelays(TICKS); let tick = 0; let h: ReturnType<typeof setTimeout>;
    const step = () => {
      tick += 1;
      setSlots(prev => stepSlots(prev, tick, pool.current, rand));
      if (tick >= TICKS) { setFinal(true); return; }
      h = setTimeout(step, delays[tick]);
    };
    h = setTimeout(step, delays[0]);
    const end = setTimeout(finish, hold ? 1e9 : durationMs);
    return () => { clearTimeout(h); clearTimeout(end); };
  }, [reduced, finish, rand, durationMs, hold]);

  // Bấm hoặc phím bất kỳ → bỏ qua, và NUỐT sự kiện (capture + stopImmediatePropagation) để Esc/Enter không rơi xuống Onboarding hay phím tắt bên dưới.
  // Tổ hợp ⌘/Ctrl/Alt và phím modifier đơn lẻ không tính là "bỏ qua" và không bị nuốt.
  // Đăng ký trên WINDOW (capture) và bằng useLayoutEffect: window capture luôn chạy trước mọi listener của document (kể cả sự kiện gửi thẳng vào document, nơi WebKit chạy theo thứ tự đăng ký),
  // còn layout effect gắn listener ngay lúc commit — không có khoảng hở giữa lúc splash hiện trong DOM và lúc nó bắt được phím (máy tải nặng).
  useLayoutEffect(() => {
    const skip = (e: Event) => {
      if (e instanceof KeyboardEvent && (e.metaKey || e.ctrlKey || e.altKey || e.key === 'Shift' || e.key === 'Control' || e.key === 'Alt' || e.key === 'Meta')) return;
      e.preventDefault(); e.stopPropagation(); e.stopImmediatePropagation();
      if (!(e instanceof KeyboardEvent && e.repeat)) finish();
    };
    window.addEventListener('keydown', skip, true); window.addEventListener('keyup', swallowUp, true); window.addEventListener('pointerdown', skip, true);
    return () => { window.removeEventListener('keydown', skip, true); window.removeEventListener('keyup', swallowUp, true); window.removeEventListener('pointerdown', skip, true); };
  }, [finish]);

  // Vạch baseline / x-height khớp thật: đo vị trí baseline của dòng chữ ở frame cuối.
  const wm = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (!final || !stage.current || !mark.current || !wm.current) return;
    const s = stage.current.getBoundingClientRect(); const b = mark.current.getBoundingClientRect();
    const size = parseFloat(getComputedStyle(wm.current).fontSize);
    if (!(s.height > 0) || !Number.isFinite(size)) return;
    setGuides({ base: b.bottom - s.top, x: b.bottom - s.top - size * xRatio(SANS) });
  }, [final]);

  return (
    <div className="fa-splash" role="status" aria-label={t('splash.label')} data-testid="splash" data-final={final} data-leaving={leaving}>
      <div className="fa-splash-stage" ref={stage}>
        {guides && <>
          <i className="fa-splash-guide x" data-testid="splash-guide-x" style={{ top: guides.x }} />
          <i className="fa-splash-guide base" data-testid="splash-guide-base" style={{ top: guides.base }} />
        </>}
        {/* Mỗi ký tự là một span riêng nhưng SÁT nhau (không khoảng trắng); căn theo baseline chung. */}
        <div className="fa-wm" ref={wm} aria-hidden>
          {[...WORD].map((ch, i) => {
            const { font } = slots[i];
            // Mọi font đảo đều chỉnh theo x-height của phần Atlas (cỡ gốc 1em = --wm) nên nhìn đều nhau khi đảo.
            const size = sizeForXHeight(xRatio(SANS), font.ratio);
            return <span key={i} data-testid="splash-char" data-family={font.family} data-locked={slots[i].locked} data-part={i < SERIF_PART.length ? 'serif' : 'sans'}
              style={{ fontFamily: `"${font.family}", var(--font-ui)`, fontWeight: font.weight, fontSize: `${size}em` }}>{ch}</span>;
          })}
          <span className="fa-wm-base" ref={mark} />
        </div>
      </div>
      <p className="fa-splash-hint">{t('splash.skip')}</p>
      <span className="sr-only">{SERIF_PART}{SANS_PART}</span>
    </div>
  );
}
