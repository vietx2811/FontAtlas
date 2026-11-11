import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CmdError } from '../../../api';
import { RemoveSourceDialog } from '../RemoveSourceDialog';
import { useScanStore } from '../../../store/scan';

const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
afterEach(cleanup);
const report = (o = {}) => ({ removedLocations: 12, facesLeftWithoutLocation: 9, casesAffected: 2, collectionsAffected: 3, tagsAffected: 4, activeFonts: 5, ...o });
async function setup(rep = report()) {
  const api = createMockApi();
  const source = (await api.listSources())[0];
  vi.spyOn(api, 'removeSourcePreview').mockResolvedValue(rep);
  const removeSource = vi.spyOn(api, 'removeSource').mockResolvedValue(rep);
  const onRemoved = vi.fn(); const onClose = vi.fn();
  render(<RemoveSourceDialog api={api} source={source} onClose={onClose} onRemoved={onRemoved} />); await settle();
  return { api, source, removeSource, onRemoved, onClose };
}

describe('RemoveSourceDialog', () => {
  it('shows the real numbers from the preview, says no file is deleted, warns about active fonts, and nothing is removed until confirm', async () => {
    const { removeSource } = await setup();
    const box = screen.getByTestId('rmsrc');
    expect(box.textContent).toContain('không xoá file font nào');
    const f = (k: string) => box.querySelector(`[data-fact="${k}"]`)?.textContent;
    expect(f('locations')).toContain('12 vị trí');
    expect(f('orphans')).toContain('9 font chỉ có ở nguồn này');
    expect(f('cases')).toContain('2 Case');
    expect(f('collections')).toContain('3 dự án');
    expect(f('tags')).toContain('4 thẻ');
    expect(f('active')).toContain('5 font');
    expect(screen.getByTestId('rmsrc-active-note').dataset.kind).toBe('warn');
    expect(screen.getByTestId('rmsrc-readd')).toBeTruthy();
    expect(removeSource).not.toHaveBeenCalled();
  });
  it('omits lines for zero counts', async () => {
    await setup(report({ facesLeftWithoutLocation: 0, casesAffected: 0, collectionsAffected: 0, tagsAffected: 0, activeFonts: 0 }));
    const box = screen.getByTestId('rmsrc');
    expect(box.querySelectorAll('li')).toHaveLength(1);
    expect(screen.queryByTestId('rmsrc-readd')).toBeNull();
  });
  it('confirm removes once (double click guarded) and hands the report back', async () => {
    const { removeSource, onRemoved } = await setup();
    fireEvent.click(screen.getByTestId('rmsrc-confirm')); fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
    expect(removeSource).toHaveBeenCalledTimes(1);
    expect(onRemoved).toHaveBeenCalledWith(report());
  });
  it('cancel closes without removing', async () => {
    const { removeSource, onClose } = await setup();
    fireEvent.click(screen.getByTestId('rmsrc-cancel')); await settle();
    expect(onClose).toHaveBeenCalled(); expect(removeSource).not.toHaveBeenCalled();
  });
  it('a failing preview shows the error and keeps Gỡ disabled until a retry works; a failing remove keeps the dialog open with the message', async () => {
    const api = createMockApi();
    const source = (await api.listSources())[0];
    const prev = vi.spyOn(api, 'removeSourcePreview').mockRejectedValueOnce(new CmdError('busy', 'scan running'));
    const rem = vi.spyOn(api, 'removeSource').mockRejectedValueOnce(new Error('db locked'));
    const onRemoved = vi.fn();
    render(<RemoveSourceDialog api={api} source={source} onClose={() => {}} onRemoved={onRemoved} />); await settle();
    expect(screen.getByTestId('rmsrc-error').textContent).toContain('Không tính được');
    expect(screen.getByTestId('rmsrc-confirm').getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
    expect(rem).not.toHaveBeenCalled();
    prev.mockResolvedValue(report());
    fireEvent.click(screen.getByTestId('rmsrc-retry')); await settle();
    expect(screen.getByTestId('rmsrc-confirm').getAttribute('aria-disabled')).toBeNull();
    fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
    expect(screen.getByTestId('rmsrc-error').textContent).toContain('Không gỡ được nguồn: db locked');
    expect(onRemoved).not.toHaveBeenCalled();
    expect(screen.getByTestId('rmsrc')).toBeTruthy();
  });
  it('numbers that changed between opening and confirming: nothing is removed, the new numbers are shown, a second confirm removes', async () => {
    const api = createMockApi();
    const source = (await api.listSources())[0];
    const prev = vi.spyOn(api, 'removeSourcePreview').mockResolvedValue(report());
    const rem = vi.spyOn(api, 'removeSource').mockResolvedValue(report({ removedLocations: 30 }));
    const onRemoved = vi.fn();
    render(<RemoveSourceDialog api={api} source={source} onClose={() => {}} onRemoved={onRemoved} />); await settle();
    prev.mockResolvedValue(report({ removedLocations: 30 })); // đang quét: thêm vị trí
    fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
    expect(rem).not.toHaveBeenCalled();
    expect(screen.getByTestId('rmsrc-error').textContent).toContain('Số liệu vừa thay đổi');
    expect(screen.getByTestId('rmsrc').querySelector('[data-fact="locations"]')?.textContent).toContain('30 vị trí');
    fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
    expect(rem).toHaveBeenCalledTimes(1);
    expect(onRemoved).toHaveBeenCalled();
  });
  it('while a scan runs the dialog says the numbers may still change', async () => {
    useScanStore.setState({ scanning: true });
    await setup();
    expect(screen.getByTestId('rmsrc-scanning')).toBeTruthy();
    cleanup(); useScanStore.setState({ scanning: false });
    await setup();
    expect(screen.queryByTestId('rmsrc-scanning')).toBeNull();
  });
});
