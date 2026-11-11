// Minh hoạ cho từng bước onboarding: bản vẽ kỹ thuật phòng lab (blueprint), glyph Inter vẽ nét rồi đổ đặc, callout đánh số, đo kích thước.
// Bước quét (o4) có hình riêng trong ScanBlocks; ở đây o4 chỉ chừa khung (slot) để ScanBlocks gắn vào qua portal.
import type { ReactNode } from 'react';
import { t, type I18nKey } from '../../i18n';
import type { Step } from '../../store/onboarding';
import { CAP_HEIGHT, GLYPHS, UPM, X_HEIGHT } from './glyphOutlines';
import { Callout, Dimension, GlyphArt, Guide, Readout, Sheet, VB_SQ, VB_W, circlePath, delay, glyphWidth } from './blueprint';

export const ILLUSTRATED_STEPS = ['o1', 'o2', 'o3', 'o5', 'o6', 'o7', 'end'] as const;
type S = typeof ILLUSTRATED_STEPS[number];

const em = (units: number, size: number) => (units * size) / UPM;

function Welcome(): React.JSX.Element {
  const size = 210; const base = 236; const x = 170;
  const cap = base - em(CAP_HEIGHT, size); const xh = base - em(X_HEIGHT, size); const top = base - em(GLYPHS['Ặ'].top, size);
  const apex = x + em((GLYPHS.A.x0 + GLYPHS.A.x1) / 2, size);
  return (
    <>
      <Sheet fig="FIG.01 · ANATOMY" h={VB_SQ}>
      <Guide y={cap} name="cap" at={200} />
      <Guide y={xh} name="x-height" at={320} />
      <Guide y={base} name="baseline" at={440} tone="base" />
      <Guide y={top} name="top" at={560} x1={apex - 40} x2={apex + 40} />
      <Dimension x={150} y1={cap} y2={base} at={1000} />
      <Dimension x={124} y1={xh} y2={base} at={1100} />
      <GlyphArt ch="Ặ" x={x} y={base} size={size} at={700} fillAt={1700} />
      <Callout term="cap_height" px={apex} py={cap} cx={336} cy={cap} at={1400} />
      <Callout term="x_height" px={x + em(449, size)} py={xh} cx={336} cy={xh} at={1520} />
      <Callout term="baseline" px={x + em(51, size)} py={base} cx={336} cy={base} at={1640} />
      <Callout term="vi_marks" px={apex} py={(top + cap) / 2 - 2} cx={336} cy={top + 6} at={1760} />
      <Readout x={340} y={cap + 10} to={CAP_HEIGHT / UPM} at={1000} />
      <Readout x={340} y={xh + 10} to={X_HEIGHT / UPM} at={1100} />
      </Sheet>
    </>
  );
}

function Legacy(): React.JSX.Element {
  const size = 118; const base = 200;
  const w = glyphWidth('ẫ', size);
  return (
    <>
      <Sheet fig="FIG.02 · MIGRATION" h={VB_SQ}>
      <g className="bp-frame old" style={delay(100)}><rect x="40" y="64" width="160" height="176" rx="6" /><text x="52" y="82" className="bp-tag">v1</text></g>
      <g className="bp-frame new" style={delay(500)}><rect x="280" y="64" width="160" height="176" rx="6" /><text x="292" y="82" className="bp-tag">v2</text></g>
      <GlyphArt ch="ẫ" x={120 - w / 2} y={base} size={size} at={300} fill={false} className="old" />
      <path d="M208 152H272M264 146L272 152L264 158" className="bp-flow" pathLength={1} style={delay(900)} />
      {[0, 1, 2].map(i => <rect key={i} x="208" y="147" width="8" height="10" rx="1.5" className="bp-packet" style={delay(1300 + i * 420)} />)}
      <GlyphArt ch="ẫ" x={360 - w / 2} y={base} size={size} at={1500} fillAt={2300} />
      </Sheet>
    </>
  );
}

function Tray(): React.JSX.Element {
  const tiles = ['A', 'a', 'g', 'H', 'x', 'ê'];
  const size = 40;
  return (
    <>
      <Sheet fig="FIG.03 · SAMPLE TRAY" h={VB_SQ}>
      <path className="bp-tray" pathLength={1} style={delay(150)} d="M64 124V104a6 6 0 0 1 6-6h88l16 22h236a6 6 0 0 1 6 6V252a6 6 0 0 1-6 6H70a6 6 0 0 1-6-6z" />
      {tiles.map((ch, i) => {
        const col = i % 3; const row = Math.floor(i / 3);
        const cx = 84 + col * 108; const cy = 150 + row * 52;
        return (
          <g key={ch} data-well={i}>
            <rect x={cx} y={cy} width="92" height="42" rx="4" className="bp-well" />
            <g className="bp-drop" style={delay(700 + i * 260)}>
              <GlyphArt ch={ch} x={cx + 46 - glyphWidth(ch, size) / 2} y={cy + 30} size={size} at={900 + i * 260} fillAt={1200 + i * 260} sw={1.2} />
            </g>
          </g>
        );
      })}
      <path d="M240 36V84M232 76L240 84L248 76" className="bp-flow" pathLength={1} style={delay(300)} />
      </Sheet>
    </>
  );
}

function Seal(): React.JSX.Element {
  const size = 104; const w = glyphWidth('ự', size);
  return (
    <>
      <Sheet fig="FIG.05 · SPECIMEN LABEL" h={VB_SQ}>
      <rect x="120" y="44" width="240" height="236" rx="8" className="bp-card" style={delay(100)} />
      <circle cx="240" cy="132" r="62" className="bp-ring spin" />
      <path d={circlePath(240, 132, 52)} className="bp-ring-solid" pathLength={1} style={delay(300)} />
      <GlyphArt ch="ự" x={240 - w / 2} y={156} size={size} at={600} fillAt={1500} />
      {[[150, 224, 180], [150, 240, 130], [150, 256, 156]].map(([x, y, wd], i) => (
        <rect key={y} x={x} y={y} width={wd} height="6" rx="3" className="bp-bar" style={delay(1600 + i * 200)} />
      ))}
      <path d="M278 262c8-14 14-14 18-2s10 8 22-6" className="bp-sign" pathLength={1} style={delay(2200)} />
      </Sheet>
    </>
  );
}

function Embedding(): React.JSX.Element {
  const size = 188; const base = 236; const x = 52; const w = glyphWidth('ê', size);
  const top = base - em(GLYPHS['ê'].top, size);
  const pts: React.JSX.Element[] = [];
  for (let r = 0; r < 6; r++) for (let c = 0; c < 5; c++) {
    const px = x + 8 + c * ((w - 16) / 4); const py = top + 14 + r * ((base - top - 28) / 5);
    pts.push(<circle key={`${r}${c}`} cx={px} cy={py} r="2.2" className="bp-sample" style={delay(1000 + ((r * 5 + c) * 53) % 700)} />);
  }
  const bars = [0.5, 0.82, 0.35, 0.94, 0.62, 0.28, 0.76, 0.48];
  return (
    <>
      <Sheet fig="FIG.06 · FEATURE SCAN" h={VB_SQ}>
      <GlyphArt ch="ê" x={x} y={base} size={size} at={250} fillAt={1800} className="ghost" />
      {pts}
      <rect x={x - 8} y={top} width={w + 16} height="3" className="bp-beam sweep" style={{ ...delay(900), ['--travel' as string]: `${base - top}px` }} />
      <path d={`M${x + w + 28} 150H300M292 144L300 150L292 156`} className="bp-flow" pathLength={1} style={delay(1500)} />
      <line x1="316" x2="440" y1="236" y2="236" className="bp-guide base" />
      {bars.map((v, i) => <rect key={i} x={318 + i * 15} y={236 - v * 110} width="9" height={v * 110} className="bp-vec" style={delay(1800 + i * 90)} />)}
      </Sheet>
    </>
  );
}

function Activate(): React.JSX.Element {
  const size = 92; const w = glyphWidth('ă', size);
  return (
    <>
      <Sheet fig="FIG.07 · PROTOTYPE" h={VB_SQ}>
      <rect x="48" y="130" width="112" height="112" rx="6" className="bp-frame-box" style={delay(150)} />
      <GlyphArt ch="ă" x={104 - w / 2} y={206} size={size} at={400} fillAt={1100} className="src" />
      <Callout term="activate" px={150} py={77} cx={176} cy={46} at={1200} />
      <g className="bp-switch" style={delay(1000)}><rect x="86" y="62" width="64" height="30" rx="15" className="bp-pill" /><circle cx="101" cy="77" r="11" className="bp-knob" /></g>
      <path d="M168 186H236" className="bp-flow dash" pathLength={1} style={delay(1400)} />
      <g className="bp-window" style={delay(1500)}>
        <rect x="244" y="104" width="196" height="148" rx="8" className="bp-wire" />
        <path d="M244 128H440" className="bp-wire" />
        {[0, 1, 2].map(i => <circle key={i} cx={258 + i * 13} cy="116" r="3" className="bp-dot" />)}
        <GlyphArt ch="ă" x={342 - w / 2} y={220} size={size} at={1900} fillAt={2400} className="fly" />
        <rect x="262" y="142" width="60" height="5" rx="2.5" className="bp-bar" style={delay(2400)} />
        <rect x="360" y="142" width="62" height="5" rx="2.5" className="bp-bar" style={delay(2500)} />
      </g>
      </Sheet>
    </>
  );
}

function Done(): React.JSX.Element {
  return (
    <>
      <Sheet fig="FIG.08 · COMPLETE" h={VB_SQ}>
      <Guide y={238} name="baseline" at={200} tone="base" />
      <Guide y={238 - em(X_HEIGHT, 150)} name="x-height" at={320} />
      <GlyphArt ch="A" x={100} y={238} size={150} at={500} fillAt={1500} />
      <GlyphArt ch="a" x={100 + glyphWidth('A', 150) + 6} y={238} size={150} at={800} fillAt={1700} />
      <path d={circlePath(388, 104, 34)} className="bp-ring-solid" pathLength={1} style={delay(1600)} />
      <path d="M372 105l11 11 21-23" className="bp-tick-mark" pathLength={1} style={delay(2100)} />
      </Sheet>
    </>
  );
}

const ART: Record<S, () => React.JSX.Element> = { o1: Welcome, o2: Legacy, o3: Tray, o5: Seal, o6: Embedding, o7: Activate, end: Done };

/** Khung hình (stage) của một bước. `slotRef` chỉ dùng ở o4: nơi ScanBlocks gắn hình quét vào. */
export function Illustration({ step, slotRef }: { step: Step | S; slotRef?: (el: HTMLDivElement | null) => void }): React.JSX.Element | null {
  if (step === 'o4') return <figure className="fa-stage" data-step="o4"><div className="fa-stage-slot" ref={slotRef} /></figure>;
  const Art = (ART as Record<string, (() => React.JSX.Element) | undefined>)[step];
  if (!Art) return null;
  const node: ReactNode = <Art />;
  return (
    <figure className="fa-stage" data-step={step}>
      <svg className="fa-bp" role="img" aria-label={t(`onb.art.${step}` as I18nKey)} viewBox={`0 0 ${VB_W} ${VB_SQ}`} preserveAspectRatio="xMidYMid meet">{node}</svg>
    </figure>
  );
}
