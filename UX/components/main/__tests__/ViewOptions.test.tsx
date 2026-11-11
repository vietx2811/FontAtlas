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
  it('⌘J toggles; Esc closes and returns focus to the button; clicking outside closes', async () => {
    render(<ViewOptions />);
    fireEvent.keyDown(document, { key: 'j', code: 'KeyJ', metaKey: true });
    expect(screen.getByTestId('view-pop')).toBeTruthy();
    fireEvent.keyDown(document, { key: 'Escape' }); await settle(20);
    expect(screen.queryByTestId('view-pop')).toBeNull();
    expect(document.activeElement).toBe(screen.getByTestId('view-open'));
    fireEvent.click(screen.getByTestId('view-open'));
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId('view-pop')).toBeNull();
    expect(screen.getByRole('button', { name: 'Mở Hiển thị' })).toBeTruthy();
  });
});
