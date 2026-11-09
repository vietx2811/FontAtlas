import { t } from '../../i18n';
import { Tooltip } from '../ui/Tooltip';
import type { Group, GroupId } from './groupFilters';
import './filter.css';

export const SUMMARY_VISIBLE = 2;

/** Tóm tắt trong ô tìm: mỗi nhóm một đoạn ("Serif +1"), quá SUMMARY_VISIBLE nhóm thì gộp thành "+N". Bấm mở "Đang lọc". */
export function FilterSummary({ groups, selected, open, onOpen }: { groups: Group[]; selected: GroupId | null; open: boolean; onOpen(): void }): React.JSX.Element | null {
  if (!groups.length) return null;
  const shown = groups.slice(0, SUMMARY_VISIBLE);
  const hidden = groups.length - shown.length;
  const full = groups.map(g => `${g.label}: ${g.chips.map(c => c.label).join(', ')}`).join('\n');
  return (
    <Tooltip label={<span style={{ whiteSpace: 'pre-line' }}>{full}</span>}>
      <button type="button" className="fa-fsum" data-testid="filter-summary" aria-haspopup="dialog" aria-expanded={open} onClick={onOpen}>
        {shown.map(g => (
          <span key={g.id} className="fa-fsum-seg" data-group={g.id} data-selected={selected === g.id}>
            {g.chips[0].label}{g.chips.length > 1 ? ` +${g.chips.length - 1}` : ''}
          </span>
        ))}
        {hidden > 0 && <span className="fa-fsum-seg fa-fsum-more" data-testid="filter-summary-more">{t('fgroup.more', { n: hidden })}</span>}
      </button>
    </Tooltip>
  );
}
