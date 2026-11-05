// Minh hoạ cho từng bước onboarding: tối giản, vẽ bằng chính chữ (glyph lớn, đường x-height/baseline, dấu Việt), SVG nội tuyến,
// chỉ dùng token màu của theme nên đổi sáng/tối theo ứng dụng. Không ảnh bitmap, không emoji.
import type { ReactNode } from 'react';
import type { Step } from '../../store/onboarding';

export const ILLUSTRATED_STEPS = ['o1', 'o2', 'o3', 'o5', 'o6', 'o7', 'end'] as const;
type S = typeof ILLUSTRATED_STEPS[number];

const INK = 'var(--fg)';
const LINE = 'var(--border)';
const MUTED = 'var(--muted)';
const ACCENT = 'var(--accent)';
const FONT = 'var(--font-ui)';
const G = ({ x, y, size, c, w = 600, children }: { x: number; y: number; size: number; c?: string; w?: number; children: string }) => (
  <text x={x} y={y} fontSize={size} fontWeight={w} fill={c ?? INK} fontFamily={FONT} textAnchor="middle">{children}</text>
);
const Guide = ({ y, name, x1 = 20, x2 = 220, dash }: { y: number; name: string; x1?: number; x2?: number; dash?: boolean }) => (
  <line data-guide={name} x1={x1} x2={x2} y1={y} y2={y} stroke={name === 'baseline' ? ACCENT : LINE} strokeWidth="1" strokeDasharray={dash ? '3 3' : undefined} />
);

const ART: Record<S, { label: string; node: ReactNode }> = {
  // Chào mừng: chữ Ặ lớn giữa đường x-height và baseline.
  o1: { label: 'Chữ Ặ lớn đặt giữa đường x-height và đường nền', node: (<>
    <Guide y={30} name="cap" dash /><Guide y={52} name="x-height" dash /><Guide y={96} name="baseline" />
    <G x={120} y={96} size={92}>Ặ</G>
  </>) },
  // Dữ liệu cũ: chữ ẫ chuyển từ khung cũ sang khung mới.
  o2: { label: 'Chữ ẫ chuyển từ khung cũ sang khung mới', node: (<>
    <rect x="22" y="26" width="64" height="64" rx="10" fill="none" stroke={LINE} strokeDasharray="4 4" />
    <G x={54} y={76} size={44} c={MUTED} w={400}>ẫ</G>
    <path d="M96 58h40m-9-8l9 8-9 8" fill="none" stroke={ACCENT} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    <rect x="146" y="26" width="64" height="64" rx="10" fill="none" stroke={INK} strokeWidth="1.5" />
    <G x={178} y={76} size={44}>ẫ</G>
  </>) },
  // Thư mục: hình thư mục vẽ bằng nét, bên trong là chữ A trên đường nền.
  o3: { label: 'Thư mục chứa chữ A đặt trên đường nền', node: (<>
    <path d="M46 36h46l10 12h72a8 8 0 0 1 8 8v42a8 8 0 0 1-8 8H46a8 8 0 0 1-8-8V44a8 8 0 0 1 8-8z" fill="none" stroke={INK} strokeWidth="1.5" strokeLinejoin="round" />
    <Guide y={92} name="baseline" x1={64} x2={176} />
    <G x={120} y={92} size={46}>A</G>
  </>) },
  // Giấy phép: con dấu tròn bao quanh chữ ự.
  o5: { label: 'Con dấu tròn bao quanh chữ ự', node: (<>
    <circle cx="120" cy="60" r="44" fill="none" stroke={INK} strokeWidth="1.5" />
    <circle cx="120" cy="60" r="38" fill="none" stroke={LINE} strokeDasharray="2 4" />
    <G x={120} y={76} size={52}>ự</G>
    <path d="M96 100l-8 14 14-4 6 10 6-18m24 18l6-10 14 4-8-14" fill="none" stroke={ACCENT} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
  </>) },
  // Phân tích hình ảnh: chữ ê trong khung ngắm, các điểm lấy mẫu.
  o6: { label: 'Chữ ê trong khung ngắm với các điểm lấy mẫu', node: (<>
    <path d="M70 34v-8a4 4 0 0 1 4-4h8M158 22h8a4 4 0 0 1 4 4v8M170 86v8a4 4 0 0 1-4 4h-8M82 98h-8a4 4 0 0 1-4-4v-8" fill="none" stroke={INK} strokeWidth="1.5" strokeLinecap="round" />
    <G x={120} y={82} size={58}>ê</G>
    {[[92, 44], [120, 40], [148, 44], [90, 70], [150, 70]].map(([x, y]) => <circle key={`${x}${y}`} cx={x} cy={y} r="2.2" fill={ACCENT} />)}
  </>) },
  // Kích hoạt: công tắc bật, chữ Ă nối sang một cửa sổ ứng dụng.
  o7: { label: 'Công tắc bật nối chữ Ă với một cửa sổ ứng dụng', node: (<>
    <rect x="22" y="40" width="58" height="32" rx="16" fill={ACCENT} />
    <circle cx="64" cy="56" r="12" fill="var(--sheet)" />
    <path d="M86 56h34" fill="none" stroke={ACCENT} strokeWidth="2" strokeDasharray="3 4" strokeLinecap="round" />
    <rect x="128" y="26" width="84" height="62" rx="8" fill="none" stroke={INK} strokeWidth="1.5" />
    <path d="M128 40h84" fill="none" stroke={LINE} />
    <G x={170} y={80} size={36}>Ă</G>
  </>) },
  // Kết thúc: Aa trên đường nền với dấu tick.
  end: { label: 'Chữ Aa trên đường nền, đánh dấu hoàn tất', node: (<>
    <Guide y={86} name="baseline" x1={50} x2={190} />
    <G x={96} y={86} size={64}>A</G><G x={148} y={86} size={44} w={400}>a</G>
    <circle cx="196" cy="34" r="13" fill="var(--on)" />
    <path d="M190 34.5l4.2 4.2 7.8-8" fill="none" stroke="var(--sheet)" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
  </>) },
};

export function Illustration({ step }: { step: Step | S }): React.JSX.Element | null {
  const art = (ART as Record<string, { label: string; node: ReactNode }>)[step];
  if (!art) return null;
  return (
    <svg className="fa-illus" role="img" aria-label={art.label} viewBox="0 0 240 120" width="240" height="120">
      {art.node}
    </svg>
  );
}
