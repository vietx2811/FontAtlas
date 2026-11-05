import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ZeroState } from '../ZeroState';

describe('ZeroState', () => {
  it('is one small glyph, one line and (optionally) one action', () => {
    const onAction = vi.fn();
    const { container } = render(<ZeroState glyph="ở" text="Không có font khớp" action={{ id: 'clear-filters', label: 'Xoá bộ lọc' }} onAction={onAction} />);
    expect(container.querySelector('svg text')!.textContent).toBe('ở');
    expect(screen.getByText('Không có font khớp')).toBeTruthy();
    expect(screen.getAllByRole('button')).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: 'Xoá bộ lọc' }));
    expect(onAction).toHaveBeenCalledWith('clear-filters');
  });
  it('without an action there is no button', () => {
    render(<ZeroState glyph="A" text="Đang quét" />);
    expect(screen.queryByRole('button')).toBeNull();
  });
  it('the glyph art is decorative (hidden from screen readers), the line is the message, and only theme tokens are used', () => {
    const { container } = render(<ZeroState glyph="A" text="x" />);
    expect(container.querySelector('svg')!.getAttribute('aria-hidden')).toBe('true');
    expect(container.innerHTML).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/);
  });
});
