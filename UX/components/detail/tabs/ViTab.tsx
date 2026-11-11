import type { CSSProperties } from 'react';
import type { FaceDetailDto } from '../../../api';
import { ZeroState } from '../../zero/ZeroState';
import { Tooltip } from '../../ui/Tooltip';
import { t } from '../../../i18n';

const LOWER = 'àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ'; // i18n-ok: bộ ký tự
/** 67 chữ thường + 67 chữ hoa = 134 ký tự (UX 03 §6.2). */
export const VI_CHARS: string[] = [...LOWER, ...LOWER.toUpperCase()];
const STACK = ['Ắ Ằ Ẳ Ẵ Ặ', 'ấ ầ ẩ ẫ ậ', 'Ơ ơ Ư ư Đ đ']; // i18n-ok: bộ ký tự
const caps = (): Array<[keyof FaceDetailDto['vietnamese'], string]> => [
  ['precomposedComposite', t('vi.caps.precomposed')], ['gposMark', t('vi.caps.mark')], ['gposMkmk', t('vi.caps.mkmk')], ['gsubCcmp', t('vi.caps.ccmp')], ['dongSign', t('vi.caps.dong')],
];

export function ViTab({ detail, css }: { detail: FaceDetailDto; css: CSSProperties }): React.JSX.Element {
  const v = detail.vietnamese;
  const missing = new Set(v.missing);
  const total = v.total || VI_CHARS.length;
  const have = total - VI_CHARS.filter(c => missing.has(c)).length;
  if (have / total < 0.1) return <ZeroState compact glyph="ạ" text={t('vi.noLatin')} />;
  return (
    <>
      <div className="fa-vi-stack" data-testid="vi-stack" style={css}>{STACK.map(l => <div key={l}>{l}</div>)}</div>

      <div className="fa-vi-cov">
        <div role="meter" aria-label={t('vi.meter')} aria-valuemin={0} aria-valuemax={total} aria-valuenow={have} className="fa-vi-bar"><i style={{ width: `${(have / total) * 100}%` }} /></div>
        <span className="fa-vi-num" data-testid="vi-count">{have}/{total}</span>
      </div>

      {v.stackedMarksOverflow && (
        <div role="note" className="fa-vi-warn">
          {t('vi.overflow')}{v.stackStyle ? t('vi.stackStyle', { style: v.stackStyle }) : ''}. {t('vi.checkTight')} <code>stacked_marks_overflow</code>
        </div>
      )}

      <div className="fa-vi-grid" style={css} role="list" aria-label={t('vi.grid')}>
        {VI_CHARS.map(c => missing.has(c)
          ? <Tooltip key={c} label={t('vi.missingChar', { c })}><span role="listitem" className="fa-vi-cell miss" data-testid="vi-cell-missing" data-missing="true">{c}<small>{t('vi.missing')}</small></span></Tooltip>
          : <span key={c} role="listitem" className="fa-vi-cell" data-testid="vi-cell">{c}</span>)}
      </div>

      <ul className="fa-vi-caps">
        {caps().map(([k, label]) => <li key={k}><span aria-hidden>{v[k] ? '✓' : '✗'}</span> {label}<span className="sr-only">{v[k] ? t('vi.yes') : t('vi.no')}</span></li>)}
      </ul>
    </>
  );
}
