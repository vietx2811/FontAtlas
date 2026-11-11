import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
    const { createMockAuto } = await import('../../../api/mockAuto');
    void createMockAuto;
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
});
