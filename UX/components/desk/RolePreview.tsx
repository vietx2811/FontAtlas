import type { Api } from '../../api';
import type { ComboDto, ComboRoleDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { useFontFamily } from '../detail/useFontFamily';
import { Tooltip } from '../ui/Tooltip';
import { isHeadingLike, presetKey, roleLabel } from './deskRoles';
import { faceLabel } from '../../store/nameClash';

/** Cỡ chữ mẫu theo vai (px). Vai lạ dùng cỡ đoạn văn. */
const SIZE: Record<string, number> = { display: 40, heading: 30, subheading: 22, accent: 18, quote: 20, body: 16, caption: 12, label: 11, numerals: 18 };
const sizeOf = (r: ComboRoleDto): number => SIZE[presetKey(r) ?? ''] ?? 16;

function Slot({ api, row, text, size, testId }: { api: Api; row: FaceRowDto; text: string; size: number; testId?: string }): React.JSX.Element {
  const fam = useFontFamily(api, row);
  return <div className="fa-prev-text" data-testid={testId} style={{ fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: row.weightClass ?? undefined, fontStyle: row.isItalic ? 'italic' : undefined, fontSize: size, lineHeight: size >= 28 ? 1.15 : 1.5 }}>{text}</div>;
}

/**
 * Xem trước MỌI vai của Combo (Heading, Body, Caption… đúng bộ vai đang có), mỗi vai bằng font của nó.
 * Vai chưa có font hiện ô trống. Bấm một vai = chọn vai đó (cột giữa chuyển sang gợi ý cho vai đó).
 */
export function RolePreview({ api, combo, roles, role, selected, trial, headline, body, onSelect }: {
  api: Api; combo: ComboDto | undefined; roles: readonly ComboRoleDto[]; role: ComboRoleDto; /** Có đang chọn một vai (không phải Combo). */ selected: boolean;
  /** Font đang thử cho vai đang chọn (thay font hiện tại). */ trial: FaceRowDto | null; headline: string; body: string; onSelect(r: ComboRoleDto): void;
}): React.JSX.Element {
  const firstHeading = roles.find(isHeadingLike);
  const firstBody = roles.find(r => !isHeadingLike(r));
  return (
    <div className="fa-prev" data-testid="desk-preview">
      {roles.map(r => {
        const slot = combo?.slots.find(s => s.role === r);
        const row = (r === role && trial) || (slot && !slot.missing ? slot.row : null);
        const heading = isHeadingLike(r);
        const text = r === firstHeading || (heading && presetKey(r) !== 'quote') ? headline : presetKey(r) === 'numerals' ? '0123456789  1,234.56  €$₫%' : presetKey(r) === 'caption' || presetKey(r) === 'label' ? body.slice(0, 90) : body;
        const on = selected && r === role;
        return (
          <section key={r} className="fa-prev-role" data-testid="desk-prev-role" data-role={r} data-selected={on ? 'true' : undefined} data-empty={row ? undefined : 'true'}>
            <Tooltip label={row ? t('desk.prev.tip') : t('desk.prev.empty.tip')}>
              <button type="button" className="fa-prev-label" data-testid="desk-prev-pick" aria-pressed={on} onClick={() => onSelect(r)}>
                <span className="fa-prev-role-name">{roleLabel(r)}</span>
                <span className="fa-prev-font">{row ? faceLabel(row) : t('desk.prev.empty')}</span>
              </button>
            </Tooltip>
            {row
              ? <Slot api={api} row={row} text={text} size={sizeOf(r)} testId={r === firstHeading ? 'desk-heading' : r === firstBody ? 'desk-body' : undefined} />
              : <div className="fa-prev-empty" aria-hidden>{'＋ '}{t('desk.prev.empty')}</div>}
          </section>
        );
      })}
    </div>
  );
}
