import type { CSSProperties } from 'react';
import type { FaceDetailDto } from '../../../api';
import { ZeroState } from '../../zero/ZeroState';
import { Tooltip } from '../../ui/Tooltip';

const LOWER = 'àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ';
/** 67 chữ thường + 67 chữ hoa = 134 ký tự (UX 03 §6.2). */
export const VI_CHARS: string[] = [...LOWER, ...LOWER.toUpperCase()];
const STACK = ['Ắ Ằ Ẳ Ẵ Ặ', 'ấ ầ ẩ ẫ ậ', 'Ơ ơ Ư ư Đ đ'];
const CAPS: Array<[keyof FaceDetailDto['vietnamese'], string]> = [
  ['precomposedComposite', 'Dấu ghép sẵn'], ['gposMark', 'Đặt dấu (mark)'], ['gposMkmk', 'Dấu chồng dấu (mkmk)'], ['gsubCcmp', 'Ghép/tách dấu (ccmp)'], ['dongSign', 'Ký hiệu ₫'],
];

export function ViTab({ detail, css }: { detail: FaceDetailDto; css: CSSProperties }): React.JSX.Element {
  const v = detail.vietnamese;
  const missing = new Set(v.missing);
  const total = v.total || VI_CHARS.length;
  const have = total - VI_CHARS.filter(c => missing.has(c)).length;
  if (have / total < 0.1) return <ZeroState compact glyph="ạ" text="Font này không có chữ Latin tiếng Việt" />;
  return (
    <>
      <div className="fa-vi-stack" data-testid="vi-stack" style={css}>{STACK.map(l => <div key={l}>{l}</div>)}</div>

      <div className="fa-vi-cov">
        <div role="meter" aria-label="Ký tự tiếng Việt có trong font" aria-valuemin={0} aria-valuemax={total} aria-valuenow={have} className="fa-vi-bar"><i style={{ width: `${(have / total) * 100}%` }} /></div>
        <span className="fa-vi-num" data-testid="vi-count">{have}/{total}</span>
      </div>

      {v.stackedMarksOverflow && (
        <div role="note" className="fa-vi-warn">
          Dấu chồng có thể nhô quá đường cap/ascender{v.stackStyle ? ` (kiểu xếp: ${v.stackStyle})` : ''}. Kiểm tra khi đặt dòng sát nhau. <code>stacked_marks_overflow</code>
        </div>
      )}

      <div className="fa-vi-grid" style={css} role="list" aria-label="134 ký tự tiếng Việt">
        {VI_CHARS.map(c => missing.has(c)
          ? <Tooltip key={c} label={`Font không có ký tự ${c}`}><span role="listitem" className="fa-vi-cell miss" data-testid="vi-cell-missing" data-missing="true">{c}<small>thiếu</small></span></Tooltip>
          : <span key={c} role="listitem" className="fa-vi-cell" data-testid="vi-cell">{c}</span>)}
      </div>

      <ul className="fa-vi-caps">
        {CAPS.map(([k, label]) => <li key={k}><span aria-hidden>{v[k] ? '✓' : '✗'}</span> {label}<span className="sr-only">{v[k] ? ': có' : ': không'}</span></li>)}
      </ul>
    </>
  );
}
