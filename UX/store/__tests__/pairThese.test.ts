import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from '../ui';

const initial = useUiStore.getState();
const f = (n: number) => ({ hashHex: String(n), faceIndex: 0 });
beforeEach(() => useUiStore.setState(initial, true));

describe('Pair these screen state', () => {
  it('opens with 2–4 faces (cuts extra) and closes', () => {
    useUiStore.getState().openPairThese([f(1), f(2), f(3), f(4), f(5)]);
    expect(useUiStore.getState().pairThese).toHaveLength(4);
    useUiStore.getState().closePairThese();
    expect(useUiStore.getState().pairThese).toBeNull();
  });
  it('ignores a single face', () => {
    useUiStore.getState().openPairThese([f(1)]);
    expect(useUiStore.getState().pairThese).toBeNull();
  });
  it('is exclusive with Compare and closed by sidebar navigation', () => {
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().openPairThese([f(1), f(2)]);
    expect(useUiStore.getState().compare).toBeNull();
    useUiStore.getState().openCompare([f(1), f(2)]);
    expect(useUiStore.getState().pairThese).toBeNull();
    useUiStore.getState().openPairThese([f(1), f(2)]);
    useUiStore.getState().setView('star');
    expect(useUiStore.getState().pairThese).toBeNull();
  });
});
