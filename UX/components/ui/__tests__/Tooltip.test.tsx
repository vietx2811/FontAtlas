import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TOOLTIP_DELAY_MS, Tooltip } from '../Tooltip';

const tip = () => screen.queryByRole('tooltip');
const advance = (ms: number) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });

describe('Tooltip', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('appears after ~450 ms of hover, not before, and hides on leave', async () => {
    render(<Tooltip label="Cài đặt" shortcut="⌘,"><button type="button" aria-label="Cài đặt" /></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn);
    await advance(TOOLTIP_DELAY_MS - 50);
    expect(tip()).toBeNull();
    await advance(60);
    expect(tip()).toHaveTextContent('Cài đặt');
    expect(tip()).toHaveTextContent('⌘,');
    fireEvent.pointerLeave(btn);
    expect(tip()).toBeNull();
  });

  it('a pointer that leaves before the delay never shows it', async () => {
    render(<Tooltip label="X"><button type="button">x</button></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn); await advance(200); fireEvent.pointerLeave(btn); await advance(1000);
    expect(tip()).toBeNull();
  });

  it('shows immediately on keyboard focus, links with aria-describedby, and Esc hides it', async () => {
    render(<Tooltip label="Đóng chi tiết"><button type="button">×</button></Tooltip>);
    const btn = screen.getByRole('button');
    expect(btn).not.toHaveAttribute('aria-describedby');
    act(() => btn.focus());
    expect(tip()).toHaveTextContent('Đóng chi tiết');
    expect(btn.getAttribute('aria-describedby')).toBe(tip()!.id);
    fireEvent.keyDown(btn, { key: 'Escape' });
    expect(tip()).toBeNull();
    expect(btn).not.toHaveAttribute('aria-describedby');
  });

  it('hides on click and on blur', async () => {
    render(<Tooltip label="A"><button type="button">a</button></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).not.toBeNull();
    fireEvent.pointerDown(btn);
    expect(tip()).toBeNull();
    act(() => btn.focus()); expect(tip()).not.toBeNull();
    act(() => btn.blur()); expect(tip()).toBeNull();
  });

  it('only shows for a truncated element when onlyIfTruncated', async () => {
    render(<Tooltip label="Tên đầy đủ rất dài" onlyIfTruncated><span data-testid="n">Tên đầy…</span></Tooltip>);
    const el = screen.getByTestId('n');
    fireEvent.pointerEnter(el); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).toBeNull(); // vừa khung → không cần
    fireEvent.pointerLeave(el);
    Object.defineProperty(el, 'scrollWidth', { configurable: true, value: 200 });
    Object.defineProperty(el, 'clientWidth', { configurable: true, value: 80 });
    fireEvent.pointerEnter(el); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).toHaveTextContent('Tên đầy đủ rất dài');
  });

  it('flips below when there is no room above, and stays inside the window', async () => {
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 400 });
    const rect = (top: number, left: number) => ({ top, left, right: left + 20, bottom: top + 20, width: 20, height: 20, x: left, y: top, toJSON: () => ({}) }) as DOMRect;
    const size = vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(120);
    const sizeH = vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(24);
    render(<Tooltip label="Gần mép trên"><button type="button">t</button></Tooltip>);
    const btn = screen.getByRole('button');
    const r = vi.spyOn(btn, 'getBoundingClientRect').mockReturnValue(rect(4, 390));
    act(() => btn.focus());
    const t = tip()!;
    expect(t.dataset.placement).toBe('bottom');
    expect(parseInt(t.style.top)).toBeGreaterThanOrEqual(24);
    expect(parseInt(t.style.left) + 120).toBeLessThanOrEqual(400 - 8);
    act(() => btn.blur());
    r.mockReturnValue(rect(300, 100));
    act(() => btn.focus());
    expect(tip()!.dataset.placement).toBe('top');
    size.mockRestore(); sizeH.mockRestore();
  });

  it('disabled → never shows; keeps the child own handlers', async () => {
    const onClick = vi.fn();
    const { rerender } = render(<Tooltip label="Không hiện" disabled><button type="button" onClick={onClick}>a</button></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).toBeNull();
    fireEvent.click(btn);
    expect(onClick).toHaveBeenCalledTimes(1);
    rerender(<Tooltip label="Hiện"><button type="button" onClick={onClick}>a</button></Tooltip>);
    fireEvent.pointerEnter(screen.getByRole('button')); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).toHaveTextContent('Hiện');
  });

  it('only one tooltip at a time', async () => {
    render(<><Tooltip label="Một"><button type="button">1</button></Tooltip><Tooltip label="Hai"><button type="button">2</button></Tooltip></>);
    const [a, b] = screen.getAllByRole('button');
    act(() => a.focus()); expect(screen.getAllByRole('tooltip')).toHaveLength(1);
    act(() => b.focus()); expect(screen.getAllByRole('tooltip')).toHaveLength(1);
    expect(tip()).toHaveTextContent('Hai');
  });

  it('uses the label and disabled state as of the moment it opens, not as of the hover', async () => {
    const { rerender } = render(<Tooltip label="Cũ"><button type="button">a</button></Tooltip>);
    fireEvent.pointerEnter(screen.getByRole('button'));
    await advance(200);
    rerender(<Tooltip label="Mới"><button type="button">a</button></Tooltip>);
    await advance(TOOLTIP_DELAY_MS);
    expect(tip()).toHaveTextContent('Mới');
    fireEvent.pointerLeave(screen.getByRole('button'));
    fireEvent.pointerEnter(screen.getByRole('button'));
    await advance(200);
    rerender(<Tooltip label="Mới" disabled><button type="button">a</button></Tooltip>);
    await advance(TOOLTIP_DELAY_MS);
    expect(tip()).toBeNull();
  });

  it('Esc closes a hover-opened tooltip from anywhere and does not reach other Esc handlers', async () => {
    const other = vi.fn(); document.addEventListener('keydown', other);
    render(<Tooltip label="A"><button type="button">a</button></Tooltip>);
    fireEvent.pointerEnter(screen.getByRole('button')); await advance(TOOLTIP_DELAY_MS + 10);
    expect(tip()).not.toBeNull();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(tip()).toBeNull();
    expect(other).not.toHaveBeenCalled();           // Esc đầu tiên chỉ dành cho tooltip
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(other).toHaveBeenCalledTimes(1);         // Esc tiếp theo đi tiếp như thường
    document.removeEventListener('keydown', other);
  });
  it('drops role/aria-describedby synchronously when closing, even if React has not committed yet', async () => {
    render(<Tooltip label="Đóng"><button type="button">x</button></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn); await advance(TOOLTIP_DELAY_MS + 10);
    const el = tip()!; expect(el).not.toBeNull();
    // Gọi hide() ngoài act(): DOM cũ vẫn còn một nhịp nhưng đã mất ngữ nghĩa.
    btn.dispatchEvent(new MouseEvent('pointerout', { bubbles: true }));
    expect(el.getAttribute('role')).toBeNull();
    expect(el.getAttribute('aria-hidden')).toBe('true');
    expect(btn).not.toHaveAttribute('aria-describedby');
    await act(async () => {});
    expect(tip()).toBeNull();
  });

  it('a stale open timer from a previous hover never reopens it', async () => {
    render(<Tooltip label="A"><button type="button">a</button></Tooltip>);
    const btn = screen.getByRole('button');
    fireEvent.pointerEnter(btn); await advance(TOOLTIP_DELAY_MS - 20);
    fireEvent.pointerLeave(btn);
    await advance(2000);
    expect(tip()).toBeNull();
  });
});
