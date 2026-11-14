import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { Credits } from '../Credits';
import { CREDITS } from '../creditsData';
import { setLocale } from '../../../i18n';

afterEach(() => setLocale('vi'));

describe('Credits', () => {
  it('names Hyperglot with its Apache-2.0 licence, the licence text shipped in the app and the changes made (vi)', () => {
    render(<Credits />);
    const h = within(screen.getByTestId('credit-hyperglot'));
    expect(h.getByText('Hyperglot')).toBeTruthy();
    expect(h.getByText('Apache-2.0')).toBeTruthy();
    expect(h.getByText(/Dữ liệu ngôn ngữ: Hyperglot © Rosetta, Apache-2.0/)).toBeTruthy();
    expect(h.getByText(/LICENSE-hyperglot\.txt/)).toBeTruthy();
  });

  it('lists every credit with its licence in both languages and leaves no raw i18n key on screen', () => {
    for (const lang of ['vi', 'en'] as const) {
      setLocale(lang);
      const { container, unmount } = render(<Credits />);
      for (const c of CREDITS) {
        const li = within(screen.getByTestId(`credit-${c.id}`));
        expect(li.getByText(c.licence)).toBeTruthy();
      }
      expect(container.textContent).not.toMatch(/credits?\.[a-z]+\./);
      unmount();
    }
  });

  it('is English in en', () => {
    setLocale('en');
    render(<Credits />);
    expect(screen.getByText(/Language data: Hyperglot/)).toBeTruthy();
    expect(screen.getByText(/Full licence text/)).toBeTruthy();
  });
});
