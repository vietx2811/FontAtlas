import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CARD_FIELDS_KEY, DEFAULT_CARD_FIELDS, hydrateCardFields, parseCardFields, serializeCardFields, useCardFields } from '../../../store/cardFields';
import { ViewOptions } from '../ViewOptions';

const settle = async (ms = 0) => { await act(async () => { await new Promise(r => setTimeout(r, ms)); }); };

describe('card fields store', () => {
  beforeEach(() => { useCardFields.getState().reset(); });
  it('parses valid JSON in fixed order, drops unknown keys, and falls back to the default on garbage', () => {
    expect(parseCardFields('{"v":1,"fields":["variable","format","zzz","format"]}')).toEqual(['format', 'variable']);
    for (const bad of [null, '', 'not json', '{"v":2,"fields":["format"]}', '{"v":1}', '{"v":1,"fields":"format"}']) expect(parseCardFields(bad as string | null)).toEqual([...DEFAULT_CARD_FIELDS]);
    expect(parseCardFields('{"v":1,"fields":[]}')).toEqual([]); // người dùng tắt hết là hợp lệ
    expect(JSON.parse(serializeCardFields(['variable', 'format']))).toEqual({ v: 1, fields: ['format', 'variable'] });
  });
  it('hydrates from the setting and writes changes back debounced', async () => {
    const api = createMockApi();
    await api.setSetting(CARD_FIELDS_KEY, serializeCardFields(['format', 'license']));
    const set = vi.spyOn(api, 'setSetting');
    const stop = await hydrateCardFields(api);
    expect(useCardFields.getState().fields).toEqual(['format', 'license']);
    useCardFields.getState().toggle('variable');
    await settle(600);
    expect(set).toHaveBeenCalledWith(CARD_FIELDS_KEY, serializeCardFields(['format', 'license', 'variable']));
    stop();
  });
  it('flushes a pending change when stopped (unmount) and on pagehide, and retries after a failed write', async () => {
    const api = createMockApi();
    const set = vi.spyOn(api, 'setSetting');
    const stop = await hydrateCardFields(api);
    useCardFields.getState().toggle('license');
    stop(); // trong 400 ms: không được mất
    expect(set).toHaveBeenCalledWith(CARD_FIELDS_KEY, serializeCardFields(['format', 'lang', 'license']));
    set.mockClear();
    const stop2 = await hydrateCardFields(api);
    useCardFields.getState().toggle('variable');
    window.dispatchEvent(new Event('pagehide'));
    expect(set).toHaveBeenCalledWith(CARD_FIELDS_KEY, serializeCardFields(['format', 'lang', 'license', 'variable']));
    stop2();
    set.mockClear();
    set.mockRejectedValueOnce(new Error('db locked'));
    const stop3 = await hydrateCardFields(api);
    useCardFields.getState().toggle('format');
    window.dispatchEvent(new Event('pagehide')); await settle(5);
    window.dispatchEvent(new Event('pagehide')); await settle(5); // lần ghi trước lỗi: `written` chưa đổi nên ghi lại
    expect(set).toHaveBeenCalledTimes(2);
    stop3();
  });
});

describe('ViewOptions', () => {
  beforeEach(() => { useCardFields.getState().reset(); });
  it('opens from the button, lists the fields with defaults checked, toggles, resets', () => {
    render(<ViewOptions />);
    const btn = screen.getByTestId('view-open');
    expect(btn.getAttribute('aria-expanded')).toBe('false');
    fireEvent.click(btn);
    expect(btn.getAttribute('aria-expanded')).toBe('true');
    const box = (k: string) => document.querySelector<HTMLInputElement>(`[data-field=${k}] input`)!;
    expect(box('format').checked).toBe(true); expect(box('lang').checked).toBe(true);
    expect(box('license').checked).toBe(false); expect(box('variable').checked).toBe(false);
    expect((screen.getByTestId('view-reset') as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(box('license'));
    expect(useCardFields.getState().fields).toEqual(['format', 'lang', 'license']);
    expect((screen.getByTestId('view-reset') as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByTestId('view-reset'));
    expect(useCardFields.getState().fields).toEqual([...DEFAULT_CARD_FIELDS]);
  });
  it('⌘J opens and closes (close returns focus to the button); Esc inside the popover closes; clicking outside closes', async () => {
    render(<ViewOptions />);
    fireEvent.keyDown(document.body, { key: 'j', code: 'KeyJ', metaKey: true });
    const pop = screen.getByTestId('view-pop');
    fireEvent.keyDown(document.activeElement ?? pop, { key: 'j', code: 'KeyJ', metaKey: true }); await settle(20);
    expect(screen.queryByTestId('view-pop')).toBeNull();
    expect(document.activeElement).toBe(screen.getByTestId('view-open'));
    fireEvent.click(screen.getByTestId('view-open'));
    fireEvent.keyDown(screen.getByTestId('view-pop'), { key: 'Escape' }); await settle(20);
    expect(screen.queryByTestId('view-pop')).toBeNull();
    expect(document.activeElement).toBe(screen.getByTestId('view-open'));
    fireEvent.click(screen.getByTestId('view-open'));
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId('view-pop')).toBeNull();
  });

  it('⌘J is ignored while typing in an input and while another dialog is open', () => {
    render(<><input data-testid="field" /><ViewOptions /></>);
    const field = screen.getByTestId('field');
    fireEvent.keyDown(field, { key: 'j', code: 'KeyJ', metaKey: true });
    expect(screen.queryByTestId('view-pop')).toBeNull();
    const dlg = document.createElement('div'); dlg.setAttribute('role', 'dialog'); document.body.appendChild(dlg);
    fireEvent.keyDown(document.body, { key: 'j', code: 'KeyJ', metaKey: true });
    expect(screen.queryByTestId('view-pop')).toBeNull();
    dlg.remove();
    fireEvent.keyDown(document.body, { key: 'j', code: 'KeyJ', metaKey: true, isComposing: true });
    expect(screen.queryByTestId('view-pop')).toBeNull();
  });

  it('Esc is handled locally: an Esc elsewhere (another sheet) is not swallowed and does not close the popover', () => {
    render(<ViewOptions />);
    fireEvent.click(screen.getByTestId('view-open'));
    let seen = false; const on = () => { seen = true; }; document.addEventListener('keydown', on);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    document.removeEventListener('keydown', on);
    expect(seen).toBe(true);
    expect(screen.getByTestId('view-pop')).toBeTruthy();
  });

  it('a tooltip (portal) is not "outside"; Tab wraps inside; focus moving away closes', () => {
    render(<><button data-testid="other">x</button><ViewOptions /></>);
    fireEvent.click(screen.getByTestId('view-open'));
    const tip = document.createElement('div'); tip.setAttribute('role', 'tooltip'); document.body.appendChild(tip);
    fireEvent.pointerDown(tip);
    expect(screen.getByTestId('view-pop')).toBeTruthy();
    tip.remove();
    const boxes = [...document.querySelectorAll<HTMLElement>('.fa-viewpop input:not(:disabled), .fa-viewpop select:not(:disabled), .fa-viewpop button:not(:disabled)')];
    boxes.at(-1)!.focus();
    fireEvent.keyDown(boxes.at(-1)!, { key: 'Tab' });
    expect(document.activeElement).toBe(boxes[0]);
    const other = screen.getByTestId('other'); other.focus();
    fireEvent.blur(boxes[0], { relatedTarget: other });
    expect(screen.queryByTestId('view-pop')).toBeNull();
  });

});
