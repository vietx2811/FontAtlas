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

describe('compare session (H2/H3/H4)', () => {
  const k = (n: number) => `${n}:0`;
  it('survives leaving the Compare page and can be resumed', () => {
    useUiStore.getState().openCompare([f(1), f(2), f(3)]);
    useUiStore.getState().setView('star');
    expect(useUiStore.getState().compare).toBeNull();
    expect(useUiStore.getState().compareSession?.faces).toHaveLength(3);
    useUiStore.getState().resumeCompare();
    expect(useUiStore.getState().compare).toEqual([f(1), f(2), f(3)]);
  });
  it('a new openCompare replaces the session and clears pins', () => {
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().toggleComparePin(k(2));
    useUiStore.getState().openCompare([f(4), f(5)]);
    expect(useUiStore.getState().compareSession?.pinned).toEqual([]);
  });
  it('pins keep pin order, can be unpinned, and ignore unknown faces', () => {
    useUiStore.getState().openCompare([f(1), f(2), f(3)]);
    const s = useUiStore.getState();
    s.toggleComparePin(k(3)); s.toggleComparePin(k(2)); s.toggleComparePin(k(9));
    expect(useUiStore.getState().compareSession?.pinned).toEqual([k(3), k(2)]);
    useUiStore.getState().toggleComparePin(k(3));
    expect(useUiStore.getState().compareSession?.pinned).toEqual([k(2)]);
  });
  it('add stops at the cap and ignores duplicates; remove stops at two and drops pin/overlay', () => {
    useUiStore.getState().openCompare(Array.from({ length: 7 }, (_, i) => f(i)));
    expect(useUiStore.getState().addCompareFace(f(0))).toBe(false);
    expect(useUiStore.getState().addCompareFace(f(7))).toBe(true);
    expect(useUiStore.getState().addCompareFace(f(8))).toBe(false);
    expect(useUiStore.getState().compare).toHaveLength(8);
    useUiStore.getState().toggleComparePin(k(7)); useUiStore.getState().toggleCompareOverlayFace(k(7));
    useUiStore.getState().removeCompareFace(k(7));
    const cs = useUiStore.getState().compareSession!;
    expect(cs.faces).toHaveLength(7); expect(cs.pinned).toEqual([]); expect(cs.overlayIds).toEqual([]);
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().removeCompareFace(k(2));
    expect(useUiStore.getState().compareSession!.faces).toHaveLength(2);
  });
  it('add/remove while the page is closed updates the session only', () => {
    useUiStore.getState().openCompare([f(1), f(2)]);
    useUiStore.getState().closeCompare();
    useUiStore.getState().addCompareFace(f(3));
    expect(useUiStore.getState().compare).toBeNull();
    expect(useUiStore.getState().compareSession!.faces).toHaveLength(3);
  });
  it('overlay toggles are capped at four', () => {
    useUiStore.getState().openCompare(Array.from({ length: 6 }, (_, i) => f(i)));
    for (let i = 0; i < 6; i++) useUiStore.getState().toggleCompareOverlayFace(k(i));
    expect(useUiStore.getState().compareSession!.overlayIds).toEqual([k(0), k(1), k(2), k(3)]);
    useUiStore.getState().setCompareOverlay(true);
    expect(useUiStore.getState().compareSession!.overlayOn).toBe(true);
  });
});
