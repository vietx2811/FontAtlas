import type { CSSProperties } from 'react';

export const WATERFALL_SIZES = [48, 32, 24, 18, 14, 11];
export const PARAGRAPH = 'Hà Nội mùa thu, cây cơm nguội vàng, cây bàng lá đỏ, nằm kề bên nhau, phố xưa nhà cổ, mái ngói thâm nâu.'; // i18n-ok: đoạn mẫu tiếng Việt

type Props = { sample: string; css: CSSProperties };

export function SpecimenTab({ sample, css }: Props): React.JSX.Element {
  return (
    <>
      <div className="fa-big" data-testid="specimen-big" style={css}>{sample.trim() || 'Phở bò Hà Nội' /* i18n-ok: chữ mẫu */}</div>
      <p className="fa-para" data-testid="specimen-para" style={css}>{PARAGRAPH}</p>
      <div className="fa-waterfall" style={css}>
        {WATERFALL_SIZES.map(s => (
          <div key={s} className="fa-wf-row" data-testid="waterfall-row" data-size={s}>
            <span className="fa-wf-size">{s}</span>
            <span data-sample style={{ fontSize: s }}>Đẹp như Hà Nội /* i18n-ok: chữ mẫu */</span>
          </div>
        ))}
      </div>
    </>
  );
}
