import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { Modal } from '../Modal';

function Host({ onClose = () => {} }: { onClose?: () => void }) {
  const [open, setOpen] = useState(false);
  return <>
    <button type="button" onClick={() => setOpen(true)}>open</button>
    {open && <Modal title="T" onClose={() => { onClose(); setOpen(false); }}><button type="button">one</button><button type="button">two</button></Modal>}
  </>;
}

describe('Modal', () => {
  it('moves focus into the dialog and gives it back to the opener when it closes', () => {
    render(<Host />);
    const opener = screen.getByText('open'); opener.focus();
    fireEvent.click(opener);
    expect(document.activeElement).toBe(screen.getByText('one'));
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it('Esc closes even when focus is on <body>', () => {
    const close = vi.fn();
    render(<Host onClose={close} />);
    fireEvent.click(screen.getByText('open'));
    (document.activeElement as HTMLElement).blur();
    expect(document.activeElement).toBe(document.body);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(close).toHaveBeenCalledTimes(1);
  });

  it('Tab wraps inside the dialog in both directions and pulls stray focus back in', () => {
    render(<Host />);
    fireEvent.click(screen.getByText('open'));
    const [one, two] = [screen.getByText('one'), screen.getByText('two')];
    two.focus(); fireEvent.keyDown(two, { key: 'Tab' });
    expect(document.activeElement).toBe(one);
    one.focus(); fireEvent.keyDown(one, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(two);
    screen.getByText('open').focus(); fireEvent.keyDown(document.body, { key: 'Tab' });
    expect(document.activeElement).toBe(one);
  });

  it('the scrim closes only on a full click that started on it, not on a drag that ends there', () => {
    const close = vi.fn();
    render(<Host onClose={close} />);
    fireEvent.click(screen.getByText('open'));
    const scrim = screen.getByRole('dialog').parentElement!;
    // nhấn trong hộp rồi nhả ngoài nền (kéo chọn chữ): click đổ về nền nhưng không bắt đầu ở nền
    fireEvent.mouseDown(screen.getByText('one')); fireEvent.click(scrim);
    expect(close).not.toHaveBeenCalled();
    fireEvent.mouseDown(scrim); fireEvent.click(scrim);
    expect(close).toHaveBeenCalledTimes(1);
  });

  it('with two dialogs open only the top one takes Esc', () => {
    const [a, b] = [vi.fn(), vi.fn()];
    render(<><Modal title="A" onClose={a}>a</Modal><Modal title="B" onClose={b}>b</Modal></>);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(a).not.toHaveBeenCalled(); expect(b).toHaveBeenCalledTimes(1);
  });
});
