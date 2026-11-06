import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Menu, type MenuEntry } from '../Menu';

function mount(entries: MenuEntry[], onClose = vi.fn()) {
  render(<><button type="button">origin</button><Menu entries={entries} anchor={{ x: 10, y: 10 }} onClose={onClose} /></>);
  return { onClose, menu: screen.getByRole('menu') };
}
const items = () => screen.getAllByRole('menuitem');
const key = (el: Element, k: string, init: KeyboardEventInit = {}) => fireEvent.keyDown(el, { key: k, ...init });

describe('Menu', () => {
  it('arrow keys move through all items (wrapping, disabled ones focusable), Enter runs and closes', () => {
    const a = vi.fn(); const c = vi.fn();
    const { onClose, menu } = mount([{ id: 'a', label: 'A', onSelect: a }, { id: 'b', label: 'B', disabled: true, reason: 'Không áp dụng' }, { type: 'sep' }, { id: 'c', label: 'C', onSelect: c }]);
    expect(document.activeElement).toBe(menu);
    key(menu, 'ArrowDown'); expect(document.activeElement).toBe(items()[0]);
    key(document.activeElement!, 'ArrowDown'); expect(document.activeElement).toBe(items()[1]); // mục aria-disabled vẫn focus được (APG)
    key(document.activeElement!, 'Enter'); expect(onClose).not.toHaveBeenCalled(); // …nhưng Enter không chạy nó
    key(document.activeElement!, 'ArrowDown'); expect(document.activeElement).toBe(items()[2]);
    key(document.activeElement!, 'ArrowDown'); expect(document.activeElement).toBe(items()[0]); // vòng lại
    key(document.activeElement!, 'ArrowUp'); expect(document.activeElement).toBe(items()[2]);
    key(document.activeElement!, 'Enter');
    expect(c).toHaveBeenCalledTimes(1); expect(a).not.toHaveBeenCalled(); expect(onClose).toHaveBeenCalled();
  });

  it('a disabled item does nothing when clicked', () => {
    const run = vi.fn();
    const { onClose } = mount([{ id: 'x', label: 'X', disabled: true, onSelect: run }]);
    fireEvent.click(items()[0]);
    expect(run).not.toHaveBeenCalled(); expect(onClose).not.toHaveBeenCalled();
    expect(items()[0]).toHaveAttribute('aria-disabled', 'true');
  });

  it('→ opens a submenu and focuses its first item; ← and Esc close only the submenu', () => {
    const pick = vi.fn();
    const { onClose, menu } = mount([{ id: 'p', label: 'Thêm vào dự án', items: [{ id: 'one', label: 'Dự án 1', onSelect: pick }, { id: 'new', label: 'Dự án mới…' }] }, { id: 'q', label: 'Q' }]);
    key(menu, 'ArrowDown');
    const parent = items()[0];
    expect(parent).toHaveAttribute('aria-haspopup', 'menu');
    key(parent, 'ArrowRight');
    expect(parent).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getAllByRole('menu')).toHaveLength(2);
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: /Dự án 1/ }));
    key(document.activeElement!, 'ArrowLeft');
    expect(screen.getAllByRole('menu')).toHaveLength(1);
    expect(document.activeElement).toBe(parent);
    key(parent, 'ArrowRight');
    key(document.activeElement!, 'Escape');
    expect(screen.getAllByRole('menu')).toHaveLength(1); expect(onClose).not.toHaveBeenCalled();
    key(parent, 'ArrowRight');
    key(document.activeElement!, 'Enter');
    expect(pick).toHaveBeenCalledTimes(1); expect(onClose).toHaveBeenCalled();
  });

  it('Esc closes the menu and returns focus to the origin', () => {
    const origin = document.createElement('button'); document.body.appendChild(origin);
    const onClose = vi.fn();
    render(<Menu entries={[{ id: 'a', label: 'A' }]} anchor={{ x: 0, y: 0 }} onClose={onClose} returnFocus={origin} />);
    key(screen.getByRole('menu'), 'Escape');
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(origin);
    origin.remove();
  });

  it('closes on an outside press and on scroll, but not on a press inside', () => {
    const { onClose, menu } = mount([{ id: 'a', label: 'A' }]);
    fireEvent.pointerDown(menu); expect(onClose).not.toHaveBeenCalled();
    fireEvent.pointerDown(screen.getByText('origin')); expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes when the page scrolls', () => {
    const { onClose } = mount([{ id: 'a', label: 'A' }]);
    act(() => { fireEvent.scroll(document.body); });
    expect(onClose).toHaveBeenCalled();
  });

  it('keyboard-opened menu focuses the first item', () => {
    render(<Menu entries={[{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }]} anchor={{ x: 0, y: 0 }} onClose={() => {}} focusFirst />);
    expect(document.activeElement).toBe(items()[0]);
  });

  it('wheel/scroll inside the menu or a submenu does not close it; outside it does', () => {
    const { onClose, menu } = mount([{ id: 'p', label: 'Dự án', items: [{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }] }]);
    key(menu, 'ArrowDown'); key(items()[0], 'ArrowRight');
    const subItem = screen.getByRole('menuitem', { name: 'A' });
    fireEvent.wheel(subItem); fireEvent.wheel(menu); fireEvent.scroll(subItem.closest('.fa-menu2')!);
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.wheel(screen.getByText('origin'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('an aria-disabled item shows its reason as a tooltip when reached with the keyboard', () => {
    mount([{ id: 'a', label: 'A' }, { id: 'f', label: 'Hiện trong Finder', disabled: true, reason: 'Chọn một font để hiện trong Finder' }]);
    const menu = screen.getByRole('menu');
    key(menu, 'ArrowDown'); key(document.activeElement!, 'ArrowDown');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Hiện trong Finder' }));
    expect(screen.getByRole('tooltip')).toHaveTextContent('Chọn một font để hiện trong Finder');
  });
});
