import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from '../ui';

const initial = useUiStore.getState();
const f = (n: number) => ({ hashHex: String(n), faceIndex: 0 });
beforeEach(() => useUiStore.setState(initial, true));

describe('compare screen state', () => {
  it('opens with 2–8 faces and closes', () => {
    useUiStore.getState().openCompare([f(1), f(2)]);
    expect(useUiStore.getState().compare).toEqual([f(1), f(2)]);
    useUiStore.getState().closeCompare();
    expect(useUiStore.getState().compare).toBeNull();
  });
  it('ignores fewer than two faces and keeps at most eight', () => {
    useUiStore.getState().openCompare([f(1)]);
    expect(useUiStore.getState().compare).toBeNull();
    useUiStore.getState().openCompare(Array.from({ length: 10 }, (_, i) => f(i)));
    expect(useUiStore.getState().compare).toHaveLength(8);
  });
  it('a sidebar navigation closes it', () => {
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().setView('star');
    expect(useUiStore.getState().compare).toBeNull();
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().setTag('3');
    expect(useUiStore.getState().compare).toBeNull();
  });
});
