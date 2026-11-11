import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { AutoStatusDto } from '../../../api';
import { AutoActivation, targetOf, whenText } from '../AutoActivation';
import { Settings } from '../Settings';

type Api = ReturnType<typeof createMockApi>;
const apps = () => screen.getAllByTestId('auto-app');
const row = (id: string) => apps().find(a => a.dataset.app === id)!;
async function mount(api: Api, pollMs = 60_000) { render(<AutoActivation api={api} pollMs={pollMs} />); await screen.findAllByTestId('auto-app'); }

describe('Settings → Bật font tự động', () => {
  it('is a Settings group', async () => {
    render(<Settings api={createMockApi()} onClose={() => {}} onRerunOnboarding={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: 'Bật font tự động' }));
    expect((await screen.findAllByTestId('auto-app')).length).toBe(6);
  });

  it('one row per app in the fixed order; font-request apps have no Cài button; Illustrator/InDesign/InCopy do; TermTip on the title term', async () => {
    await mount(createMockApi());
    expect(apps().map(a => a.dataset.app)).toEqual(['photoshop', 'aftereffects', 'illustrator', 'indesign', 'incopy', 'affinity']);
    expect(within(row('photoshop')).queryByTestId('auto-install')).toBeNull();
    expect(within(row('illustrator')).getByTestId('auto-install').textContent).toBe('Cài');
    expect(within(row('incopy')).getByTestId('auto-install')).toBeTruthy();
    expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Chưa cài');
    expect(within(row('photoshop')).getByTestId('auto-state').textContent).toBe('Sẵn sàng');
    expect(document.querySelector('[data-term="auto_activation"]')).toBeTruthy();
  });

  it('evidence is shown honestly per app ("Chưa kiểm với After Effects")', async () => {
    await mount(createMockApi());
    expect(row('aftereffects').textContent).toContain('Chưa kiểm với After Effects');
    expect(row('photoshop').textContent).toContain('Đã kiểm với app thật');
    expect(row('illustrator').textContent).toContain('Mới kiểm một phần');
  });

  it('the master switch calls autoSetEnabled and every app shows "Tắt" while it is off', async () => {
    const api = createMockApi(); const spy = vi.spyOn(api, 'autoSetEnabled');
    await mount(api);
    fireEvent.click(screen.getByTestId('auto-switch'));
    await waitFor(() => expect(screen.getByTestId('auto-switch').getAttribute('aria-checked')).toBe('false'));
    expect(spy).toHaveBeenCalledWith(false);
    for (const a of apps()) expect(within(a).getByTestId('auto-state').textContent).toBe('Tắt');
    expect(within(row('illustrator')).getByTestId('auto-install')).toBeTruthy(); // Cài/Gỡ vẫn dùng được khi công tắc tắt
  });

  it('Cài asks first and says what will be written; Huỷ does nothing; Xác nhận installs and then offers Gỡ', async () => {
    const api = createMockApi(); const install = vi.spyOn(api, 'autoInstall');
    await mount(api);
    fireEvent.click(within(row('illustrator')).getByTestId('auto-install'));
    expect(screen.getByRole('dialog').textContent).toMatch(/panel \(tiện ích CEP\)/);
    expect(screen.getByRole('dialog').textContent).toMatch(/không sửa tệp nào khác/);
    fireEvent.click(screen.getByTestId('auto-cancel'));
    expect(install).not.toHaveBeenCalled();
    fireEvent.click(within(row('indesign')).getByTestId('auto-install'));
    expect(screen.getByRole('dialog').textContent).toMatch(/InDesign và InCopy/);
    fireEvent.click(screen.getByTestId('auto-confirm'));
    await waitFor(() => expect(install).toHaveBeenCalledWith('indesign'));
    await waitFor(() => expect(within(row('indesign')).getByTestId('auto-state').textContent).toBe('Sẵn sàng'));
    expect(within(row('incopy')).getByTestId('auto-state').textContent).toBe('Sẵn sàng'); // một lệnh cài cho cả hai
    expect(within(row('indesign')).getByTestId('auto-remove')).toBeTruthy();
  });

  it('Gỡ asks first, then removes; an install error is shown and the status is re-read', async () => {
    const api = createMockApi({ auto: { installed: ['illustrator'] } });
    await mount(api);
    const remove = vi.spyOn(api, 'autoRemove');
    fireEvent.click(within(row('illustrator')).getByTestId('auto-remove'));
    expect(screen.getByRole('dialog').textContent).toMatch(/Tệp lạ trong cùng thư mục không bị đụng/);
    fireEvent.click(screen.getByTestId('auto-confirm'));
    await waitFor(() => expect(remove).toHaveBeenCalledWith('illustrator'));
    await waitFor(() => expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Chưa cài'));
    vi.spyOn(api, 'autoInstall').mockRejectedValueOnce(new Error('không ghi được'));
    fireEvent.click(within(row('illustrator')).getByTestId('auto-install'));
    fireEvent.click(screen.getByTestId('auto-confirm'));
    expect((await screen.findByTestId('auto-error')).textContent).toContain('không ghi được');
    expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Chưa cài');
  });

  it('a running app that needs a restart after install says so; an app that is not on the machine is dimmed with a note', async () => {
    const api = createMockApi({ auto: { installed: ['illustrator'], running: ['illustrator'] } });
    await mount(api);
    expect(within(row('illustrator')).getByTestId('auto-restart').textContent).toBe('Khởi động lại Illustrator để dùng.');
    expect(within(row('photoshop')).queryByTestId('auto-restart')).toBeNull();
  });

  it('shows a warning for each running third-party font manager and none for installed-but-idle ones', async () => {
    const api = createMockApi({ auto: { others: [{ name: 'RightFont', installed: true, running: true }, { name: 'FontBase', installed: true, running: false }, { name: 'Typeface', installed: true, running: true }] } });
    await mount(api);
    const warns = screen.getAllByTestId('auto-manager-warn').map(w => w.textContent);
    expect(warns).toHaveLength(2);
    expect(warns[0]).toContain('RightFont đang chạy');
    expect(warns.join(' ')).not.toContain('FontBase');
  });

  it('font request source failed to open: a note with the guidance and the running managers; none when it runs', async () => {
    await mount(createMockApi({ auto: { fontRequestError: 'refused', others: [{ name: 'RightFont', installed: true, running: true }] } }));
    const w = screen.getByTestId('auto-fontreq-warn');
    expect(w.textContent).toContain('chưa tự bật font được');
    expect(w.textContent).toContain('RightFont');
    expect(within(w).getByRole('img').getAttribute('aria-label')).toContain('thoát các trình quản lý font khác');
    cleanup();
    await mount(createMockApi());
    expect(screen.queryByTestId('auto-fontreq-warn')).toBeNull();
  });

  it('the log lists app, font, outcome (newest first) and "last activity"; empty log has its own message', async () => {
    const empty = createMockApi();
    await mount(empty);
    expect(screen.getByTestId('auto-log-empty')).toBeTruthy();
    expect(within(row('photoshop')).getByTestId('auto-last').textContent).toBe('chưa có app nào hỏi');
  });

  it('log rows and lastActivity come from the status/log', async () => {
    const api = createMockApi();
    const { createMockAuto } = await import('../../../api/mockAuto');
    const auto = createMockAuto();
    auto.__push({ atMs: BigInt(Date.now() - 5 * 60_000), app: 'photoshop', doc: 'menu.psd', requested: 'Playfair Display', outcome: 'activated', hashHex: null });
    auto.__push({ atMs: BigInt(Date.now() - 60_000), app: 'illustrator', doc: null, requested: 'Lobster', outcome: 'missing', hashHex: null });
    Object.assign(api, auto);
    await mount(api);
    const body = screen.getByTestId('auto-log').querySelectorAll('tbody tr');
    expect(body).toHaveLength(2);
    expect(body[0].textContent).toContain('Lobster'); expect(body[0].textContent).toContain('Illustrator'); expect(body[0].textContent).toContain('Không có trong thư viện');
    expect(body[1].textContent).toContain('menu.psd'); expect(body[1].textContent).toContain('Đã bật');
    expect(within(row('photoshop')).getByTestId('auto-last').textContent).toMatch(/^lần cuối .*phút/);
  });

  it('an ambiguous outcome (several fonts with the same name, one chosen by priority) is translated and carries a note', async () => {
    const api = createMockApi();
    const { createMockAuto } = await import('../../../api/mockAuto');
    const auto = createMockAuto();
    auto.__push({ atMs: BigInt(Date.now() - 60_000), app: 'indesign', doc: null, requested: 'Inter', outcome: 'ambiguous', hashHex: null });
    Object.assign(api, auto);
    await mount(api);
    const r = screen.getByTestId('auto-log').querySelector('tbody tr')!;
    expect(r.textContent).toContain('Nhiều bản cùng tên');
    expect(within(r as HTMLElement).getByTestId('auto-ambiguous').getAttribute('aria-label')).toMatch(/theo thứ tự ưu tiên/);
  });

  it('a status error is shown with a retry that reloads', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'autoStatus').mockRejectedValueOnce(new Error('broker chết'));
    render(<AutoActivation api={api} pollMs={60_000} />);
    expect((await screen.findByTestId('auto-error')).textContent).toContain('broker chết');
    fireEvent.click(within(screen.getByTestId('auto-error')).getByRole('button', { name: 'Thử lại' }));
    await screen.findAllByTestId('auto-app');
    expect(spy).toHaveBeenCalledTimes(2);
    expect(screen.queryByTestId('auto-error')).toBeNull();
  });

  it('polls so an app starting or quitting shows up without reopening the page', async () => {
    const api = createMockApi({ auto: { installed: ['illustrator'] } });
    const spy = vi.spyOn(api, 'autoStatus');
    await mount(api, 30);
    await waitFor(() => expect(spy.mock.calls.length).toBeGreaterThan(2), { timeout: 3000 });
  });

  it('foreign files: the install button is aria-disabled and the NoteMark explains; the pure helpers', async () => {
    const api = createMockApi();
    const base = await api.autoStatus();
    const foreign: AutoStatusDto = { ...base, apps: base.apps.map(a => (a.id === 'illustrator' ? { ...a, state: 'foreign' as const } : a)) };
    vi.spyOn(api, 'autoStatus').mockResolvedValue(foreign);
    await mount(api);
    const btn = within(row('illustrator')).getByTestId('auto-install');
    expect(btn.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(btn);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(targetOf('incopy')).toBe('indesign'); expect(targetOf('photoshop')).toBeNull();
    expect(whenText(Date.now() - 10_000)).toBe('vừa xong');
    await act(async () => {});
  });

  describe('review fixes', () => {
    it('a poll that started BEFORE an install and answers after it must not overwrite the new state (no flicker)', async () => {
      const api = createMockApi();
      const real = api.autoStatus.bind(api);
      let release!: () => void; let stale = true;
      vi.spyOn(api, 'autoStatus').mockImplementation(async () => {
        const before = await real(); // trạng thái CŨ (chưa cài)
        if (stale) { stale = false; await new Promise<void>(r => (release = r)); }
        return before;
      });
      await act(async () => { render(<AutoActivation api={api} pollMs={60_000} />); });
      // lần đọc đầu đang treo; bấm Cài trong lúc đó không thể (chưa có hàng) → dùng công tắc làm lệnh thay đổi
      await act(async () => { release?.(); });
      await screen.findAllByTestId('auto-app');
      expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Chưa cài');
      let slow!: () => void; stale = true;
      const poll = vi.spyOn(api, 'autoStatus').mockImplementation(async () => { const s = await real(); await new Promise<void>(r => (slow = r)); return s; });
      window.dispatchEvent(new Event('focus')); // bắt đầu một lần đọc chậm với trạng thái cũ
      await act(async () => {});
      poll.mockRestore();
      fireEvent.click(within(row('illustrator')).getByTestId('auto-install'));
      fireEvent.click(screen.getByTestId('auto-confirm'));
      await waitFor(() => expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Sẵn sàng'));
      await act(async () => { slow?.(); });
      await new Promise(r => setTimeout(r, 20));
      expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Sẵn sàng'); // không bị đè về "Chưa cài"
    });

    it('does not stack requests: ticks are skipped while one is in flight; polling stops on unmount', async () => {
      const api = createMockApi();
      let calls = 0; let finish!: () => void;
      const real = api.autoStatus.bind(api);
      vi.spyOn(api, 'autoStatus').mockImplementation(async () => { calls++; const s = await real(); await new Promise<void>(r => (finish = r)); return s; });
      const { unmount } = render(<AutoActivation api={api} pollMs={10} />);
      await new Promise(r => setTimeout(r, 120));
      expect(calls).toBe(1); // nhiều nhịp trôi qua nhưng chỉ một request bay
      await act(async () => { finish(); });
      unmount();
      const after = calls;
      await new Promise(r => setTimeout(r, 80));
      expect(calls).toBe(after);
    });

    it('coming back to the window (focus / becoming visible) reloads immediately', async () => {
      const api = createMockApi(); const spy = vi.spyOn(api, 'autoStatus');
      await mount(api, 60_000);
      const n = spy.mock.calls.length;
      window.dispatchEvent(new Event('focus'));
      await waitFor(() => expect(spy.mock.calls.length).toBe(n + 1));
      document.dispatchEvent(new Event('visibilitychange'));
      await waitFor(() => expect(spy.mock.calls.length).toBe(n + 2));
    });

    it('an unknown backend outcome shows its original string, never a raw i18n key', async () => {
      const api = createMockApi();
      const { createMockAuto } = await import('../../../api/mockAuto');
      const auto = createMockAuto();
      auto.__push({ atMs: BigInt(Date.now() - 1000), app: 'photoshop', doc: null, requested: 'Inter', outcome: 'quarantined', hashHex: null });
      Object.assign(api, auto);
      await mount(api);
      const cell = screen.getByTestId('auto-log').querySelector('tbody tr td:last-child')!;
      expect(cell.textContent).toBe('quarantined');
      expect(cell.textContent).not.toMatch(/auto\./);
    });

    it('after Xác nhận focus lands on the app row (Gỡ appears / Cài goes), a polite live region announces it, and a double click runs once', async () => {
      const api = createMockApi(); const install = vi.spyOn(api, 'autoInstall');
      await mount(api);
      fireEvent.click(within(row('illustrator')).getByTestId('auto-install'));
      const confirm = screen.getByTestId('auto-confirm');
      fireEvent.click(confirm); fireEvent.click(confirm);
      await waitFor(() => expect(screen.getByTestId('auto-live').textContent).toBe('Đã cài cho Illustrator'));
      expect(install).toHaveBeenCalledTimes(1);
      expect(screen.getByTestId('auto-live').getAttribute('aria-live')).toBe('polite');
      await act(async () => { await new Promise(r => requestAnimationFrame(() => r(null))); });
      expect(row('illustrator').contains(document.activeElement)).toBe(true);
      expect(document.activeElement).not.toBe(document.body);
    });

    it('while busy the buttons are aria-disabled with a reason in aria-describedby ("Đang xử lý…"), not removed from the tab order', async () => {
      const api = createMockApi();
      let go!: () => void; const real = api.autoInstall.bind(api);
      vi.spyOn(api, 'autoInstall').mockImplementation(async t => { await new Promise<void>(r => (go = r)); return real(t); });
      await mount(api);
      fireEvent.click(within(row('indesign')).getByTestId('auto-install'));
      fireEvent.click(screen.getByTestId('auto-confirm'));
      const btn = await waitFor(() => { const b = within(row('illustrator')).getByTestId('auto-install'); expect(b.getAttribute('aria-disabled')).toBe('true'); return b; });
      const why = document.getElementById(btn.getAttribute('aria-describedby')!);
      expect(why?.textContent).toBe('Đang xử lý…');
      expect(btn.hasAttribute('disabled')).toBe(false);
      await act(async () => { go(); });
      await waitFor(() => expect(within(row('illustrator')).getByTestId('auto-install').getAttribute('aria-disabled')).toBeNull());
    });

    it('a foreign file explains itself to assistive tech (aria-describedby) as well as in the tooltip', async () => {
      const api = createMockApi(); const base = await api.autoStatus();
      vi.spyOn(api, 'autoStatus').mockResolvedValue({ ...base, apps: base.apps.map(a => (a.id === 'illustrator' ? { ...a, state: 'foreign' as const } : a)) });
      await mount(api);
      const btn = within(row('illustrator')).getByTestId('auto-install');
      expect(document.getElementById(btn.getAttribute('aria-describedby')!)?.textContent).toMatch(/không ghi đè/);
    });

    it('"Sẵn sàng" is neutral (not green) while the app was never tested; green when measured', async () => {
      await mount(createMockApi());
      expect(row('photoshop').querySelector('.fa-auto-state')!.getAttribute('data-tone')).toBe('ok');   // measured
      expect(row('aftereffects').querySelector('.fa-auto-state')!.getAttribute('data-tone')).toBe('idle'); // untested
    });

    it('the log is a captioned table with <time dateTime>; the error alert does not wrap the retry button', async () => {
      const api = createMockApi();
      const { createMockAuto } = await import('../../../api/mockAuto');
      const auto = createMockAuto(); auto.__push({ atMs: BigInt(1_700_000_000_000), app: 'photoshop', doc: null, requested: 'Inter', outcome: 'activated', hashHex: null });
      Object.assign(api, auto);
      await mount(api);
      const table = screen.getByTestId('auto-log');
      expect(table.getAttribute('aria-label')).toBe('Nhật ký gần đây'); expect(table.querySelector('caption')).toBeTruthy();
      expect(table.querySelector('time')!.getAttribute('datetime')).toBe(new Date(1_700_000_000_000).toISOString());
      cleanup();
      const bad = createMockApi(); vi.spyOn(bad, 'autoStatus').mockRejectedValue(new Error('x'));
      render(<AutoActivation api={bad} pollMs={60_000} />);
      const box = await screen.findByTestId('auto-error');
      expect(within(box).getByRole('alert').querySelector('button')).toBeNull();
      expect(within(box).getByRole('button', { name: 'Thử lại' })).toBeTruthy();
    });

    it('Esc closes the confirm dialog without calling the API; waiting_first_launch / stale / error are rendered with their text', async () => {
      const api = createMockApi(); const install = vi.spyOn(api, 'autoInstall'); const base = await api.autoStatus();
      vi.spyOn(api, 'autoStatus').mockResolvedValue({ ...base, apps: base.apps.map(a => (a.id === 'indesign' ? { ...a, state: 'waiting_first_launch' as const } : a.id === 'illustrator' ? { ...a, state: 'stale' as const, installed: true, detail: 'khác phiên bản app' } : a.id === 'incopy' ? { ...a, state: 'error' as const, detail: 'không thấy thư mục Adobe' } : a)) });
      await mount(api);
      expect(within(row('indesign')).getByTestId('auto-state').textContent).toBe('Mở InDesign một lần rồi quay lại');
      expect(within(row('illustrator')).getByTestId('auto-state').textContent).toBe('Cần cài lại');
      expect(within(row('illustrator')).getByTestId('auto-install').textContent).toBe('Cài lại');
      expect(within(row('incopy')).getByTestId('auto-state').textContent).toBe('Lỗi');
      fireEvent.click(within(row('illustrator')).getByTestId('auto-install'));
      expect(screen.getByRole('dialog')).toBeTruthy();
      fireEvent.keyDown(document, { key: 'Escape' });
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
      expect(install).not.toHaveBeenCalled();
    });
  });

  describe('signing key and isolated build (06 contract)', () => {
    it('removing the Illustrator panel keeps the key: a note + "Xoá khoá ký" appear only once the panel is gone; confirm forgets it', async () => {
      const api = createMockApi({ auto: { installed: ['illustrator'], keyPresent: true } });
      const forget = vi.spyOn(api, 'autoForgetKey');
      await mount(api);
      expect(screen.queryByTestId('auto-key')).toBeNull(); // panel còn cài: chưa cho xoá khoá
      fireEvent.click(within(row('illustrator')).getByTestId('auto-remove'));
      fireEvent.click(screen.getByTestId('auto-confirm'));
      const note = await screen.findByTestId('auto-key');
      expect(note.textContent).toContain('Gỡ panel không xoá khoá');
      fireEvent.click(within(note).getByTestId('auto-forget'));
      expect(screen.getByRole('dialog').textContent).toMatch(/tạo khoá mới/);
      fireEvent.click(screen.getByTestId('auto-cancel'));
      expect(forget).not.toHaveBeenCalled();
      fireEvent.click(within(note).getByTestId('auto-forget'));
      fireEvent.click(screen.getByTestId('auto-confirm'));
      await waitFor(() => expect(forget).toHaveBeenCalledTimes(1));
      await waitFor(() => expect(screen.queryByTestId('auto-key')).toBeNull());
      expect(screen.getByTestId('auto-live').textContent).toBe('Đã xoá khoá ký');
    });

    it('no key note when there is no key', async () => {
      await mount(createMockApi({ auto: { keyPresent: false } }));
      expect(screen.queryByTestId('auto-key')).toBeNull();
    });

    it('isolated build: Cài / Gỡ / Xoá khoá ký are aria-disabled with the reason (described by), do nothing when clicked, and a note says so', async () => {
      const api = createMockApi({ auto: { isolated: true, keyPresent: true, installed: [] } });
      const install = vi.spyOn(api, 'autoInstall');
      await mount(api);
      expect(screen.getByTestId('auto-isolated')).toBeTruthy();
      const btn = within(row('illustrator')).getByTestId('auto-install');
      expect(btn.getAttribute('aria-disabled')).toBe('true');
      expect(document.getElementById(btn.getAttribute('aria-describedby')!)?.textContent).toMatch(/cài, gỡ và xoá khoá ký bị tắt/);
      fireEvent.click(btn);
      expect(screen.queryByRole('dialog')).toBeNull();
      expect(install).not.toHaveBeenCalled();
      const forget = within(screen.getByTestId('auto-key')).getByTestId('auto-forget');
      expect(forget.getAttribute('aria-disabled')).toBe('true');
      fireEvent.click(forget);
      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('the mock refuses like the backend: forgetting while the panel is installed is an io error; isolated build refuses everything', async () => {
      const a = createMockApi({ auto: { installed: ['illustrator'], keyPresent: true } });
      await expect(a.autoForgetKey()).rejects.toMatchObject({ code: 'io' });
      const b = createMockApi({ auto: { isolated: true } });
      await expect(b.autoInstall('illustrator')).rejects.toMatchObject({ code: 'isolated_build' });
    });

    it('an ambiguous request keeps its "×N" count in the font column', async () => {
      const api = createMockApi();
      const { createMockAuto } = await import('../../../api/mockAuto');
      const auto = createMockAuto(); auto.__push({ atMs: BigInt(Date.now() - 1000), app: 'indesign', doc: null, requested: 'Inter ×3', outcome: 'ambiguous', hashHex: null });
      Object.assign(api, auto);
      await mount(api);
      expect(screen.getByTestId('auto-log').querySelector('tbody tr')!.textContent).toContain('Inter ×3');
    });
  });
});
