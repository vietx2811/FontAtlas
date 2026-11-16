import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PickerBar } from '../PickerBar';
import { whyText } from '../pickerSources';
import { setLocale, t } from '../../../i18n';

describe('why.distinctiveness_low và nhãn Beta', () => {
  it('lý do distinctiveness_low có câu riêng (không bị bỏ), trung tính: điểm yếu của vai', () => {
    const txt = whyText({ code: 'distinctiveness_low', params: [] });
    expect(txt).toMatch(/điểm yếu nhất của vai/); expect(txt).not.toMatch(/xếp thấp/);
  });

  it('Beta chỉ cạnh nguồn "Hợp vai" trong Picker, có thẻ giải thích riêng (khoá term.beta, không phải pick.gate.experimental)', () => {
    render(<PickerBar source="role" onSource={() => {}} pairStyle="" onPairStyle={() => {}} disabled={{}} counts={{}} classes={[]} onClasses={() => {}} viOnly={false} onViOnly={() => {}} shown={0} hidden={0} profile={null} profileGuess={null} onProfile={() => {}} showProfile={false} />);
    const badges = screen.getAllByTestId('beta-badge');
    expect(badges).toHaveLength(1);
    expect(badges[0].closest('.fa-pick-beta')?.querySelector('[data-testid=pick-src-role]')).toBeTruthy();
    expect(badges[0].dataset.term).toBe('beta');
    expect(t('term.beta.gloss')).toMatch(/thử nghiệm/); expect(t('term.beta.gloss')).not.toBe(t('pick.gate.experimental'));
  });

  it('measure_doubtful có câu vi + en (không bị bỏ dòng, không lộ mã thô)', () => {
    const w = { code: 'measure_doubtful', params: [{ key: 'x_height', value: 0.9 }] };
    expect(whyText(w)).toMatch(/đáng ngờ/);
    setLocale('en'); expect(whyText(w)).toMatch(/unreliable/); expect(whyText(w)).not.toMatch(/measure_doubtful/);
  });
});
