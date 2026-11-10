import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from '../../../store/ui';
import { FILTER_HISTORY_LIMIT, isEditableTarget, useFilterHistory } from '../useFilterHistory';

const initial = useUiStore.getState();
let api: ReturnType<typeof useFilterHistory>;
function Harness() { api = useFilterHistory(); return <input aria-label="q" />; }
const set = (f: object) => act(() => { useUiStore.getState().setFilters(f); });
const filters = () => useUiStore.getState().filters;

describe('useFilterHistory', () => {
  beforeEach(() => { useUiStore.setState(initial, true); });

  it('undo restores the previous filters and does not record itself', () => {
    render(<Harness />);
    set({ italic: true }); set({ italic: true, classes: ['serif'] });
    expect(api.canUndo).toBe(true);
    act(() => { api.undo(); });
    expect(filters()).toEqual({ italic: true });
    act(() => { api.undo(); });
    expect(filters()).toEqual({});
    expect(api.canUndo).toBe(false);
    expect(api.undo()).toBe(false);
  });

  it('keeps at most 20 steps', () => {
    render(<Harness />);
    for (let i = 0; i < FILTER_HISTORY_LIMIT + 5; i++) set({ license: [`l${i}`] });
    let n = 0; while (api.undo()) n++;
    expect(n).toBe(FILTER_HISTORY_LIMIT);
  });

  it('⌘Z undoes filters when focus is not in a text field', () => {
    render(<Harness />);
    set({ italic: true });
    fireEvent.keyDown(document.body, { key: 'z', metaKey: true });
    expect(filters()).toEqual({});
  });

  it('⌘Z in a focused text input is left to the input', () => {
    render(<Harness />);
    set({ italic: true });
    const input = screen.getByLabelText('q');
    input.focus();
    const ev = new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true, cancelable: true });
    input.dispatchEvent(ev);
    expect(filters()).toEqual({ italic: true });
    expect(ev.defaultPrevented).toBe(false);
  });

  it('isEditableTarget: text inputs yes, checkbox/button no', () => {
    const mk = (type: string) => { const i = document.createElement('input'); i.type = type; return i; };
    expect(isEditableTarget(mk('text'))).toBe(true);
    expect(isEditableTarget(mk('number'))).toBe(true);
    expect(isEditableTarget(mk('checkbox'))).toBe(false);
    expect(isEditableTarget(document.createElement('button'))).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});
