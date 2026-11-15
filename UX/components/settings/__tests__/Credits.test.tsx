import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { fireEvent, waitFor } from '@testing-library/react';
import { Credits } from '../Credits';
import { CREDITS } from '../creditsData';
import { setLocale } from '../../../i18n';

afterEach(() => setLocale('vi'));

describe('Credits', () => {
  it('names Hyperglot with its Apache-2.0 licence, the licence text shipped in the app and the changes made (vi)', () => {
    render(<Credits api={createMockApi()} />);
    const h = within(screen.getByTestId('credit-hyperglot'));
    expect(h.getByText('Hyperglot')).toBeTruthy();
    expect(h.getByText('Apache-2.0')).toBeTruthy();
    expect(h.getByText(/Dữ liệu ngôn ngữ: Hyperglot © Rosetta, Apache-2.0/)).toBeTruthy();
    expect(h.getByText(/LICENSE-hyperglot\.txt/)).toBeTruthy();
  });

  it('lists every credit with its licence in both languages and leaves no raw i18n key on screen', () => {
    for (const lang of ['vi', 'en'] as const) {
      setLocale(lang);
      const { container, unmount } = render(<Credits api={createMockApi()} />);
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
    render(<Credits api={createMockApi()} />);
    expect(screen.getByText(/Language data: Hyperglot/)).toBeTruthy();
    expect(screen.getByText(/Full licence text/)).toBeTruthy();
  });

  it('«Full licences» opens the bundled notices file; a failure becomes an error notice', async () => {
    const api = createMockApi(); const open = vi.spyOn(api, 'openThirdPartyNotices');
    setLocale('en'); render(<Credits api={api} />);
    fireEvent.click(screen.getByTestId('credits-all-licences'));
    expect(open).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId('credits-all-licences').textContent).toBe('Full licences of all components');
    open.mockRejectedValueOnce(new Error('not_found'));
    useNoticeStore.setState({ notices: [] });
    fireEvent.click(screen.getByTestId('credits-all-licences'));
    await waitFor(() => expect(useNoticeStore.getState().notices.some(n => n.kind === 'error' && /not_found/.test(n.text))).toBe(true));
  });
});
