import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SpecimenTab, WATERFALL_SIZES } from '../SpecimenTab';

describe('SpecimenTab', () => {
  it('shows the sample big, a real Vietnamese paragraph and the waterfall', () => {
    render(<SpecimenTab sample="Bún chả" css={{ fontFamily: 'X', fontWeight: 700, fontStyle: 'italic' }} />);
    expect(screen.getByTestId('specimen-big').textContent).toBe('Bún chả');
    expect(screen.getByTestId('specimen-para').textContent).toMatch(/Hà Nội mùa thu/);
    const rows = screen.getAllByTestId('waterfall-row');
    expect(rows.map(r => r.getAttribute('data-size'))).toEqual(WATERFALL_SIZES.map(String));
    expect(WATERFALL_SIZES).toEqual([48, 32, 24, 18, 14, 11]);
    expect((rows[0].querySelector('[data-sample]') as HTMLElement).style.fontSize).toBe('48px');
  });

  it('applies the chosen style to every block', () => {
    render(<SpecimenTab sample="x" css={{ fontFamily: 'X', fontWeight: 700, fontStyle: 'italic' }} />);
    for (const id of ['specimen-big', 'specimen-para']) {
      const el = screen.getByTestId(id);
      expect([el.style.fontWeight, el.style.fontStyle]).toEqual(['700', 'italic']);
    }
  });

  it('falls back to a default sample when empty', () => {
    render(<SpecimenTab sample="  " css={{}} />);
    expect(screen.getByTestId('specimen-big').textContent).toBe('Phở bò Hà Nội');
  });
});
