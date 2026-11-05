import { render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { RAIL_WIDTH, SIDEBAR_WIDTH, TRAFFIC_LIGHT_INSET, TITLEBAR_HEIGHT, WindowShell } from '../WindowShell';

const css = readFileSync(resolve(__dirname, '../windowShell.css'), 'utf8');

describe('WindowShell', () => {
  it('renders a transparent sidebar column with a drag region above it, and an opaque sheet', () => {
    render(<WindowShell sidebar={<nav>thanh bên</nav>}><p>nội dung</p></WindowShell>);
    const shell = screen.getByTestId('window-shell');
    const drag = shell.querySelector('[data-tauri-drag-region]') as HTMLElement;
    expect(drag).toBeTruthy();
    expect(drag.closest('.fa-shell-sidebar')).toBeTruthy(); // kéo cửa sổ từ hàng trên của sidebar
    expect(screen.getByText('thanh bên').closest('.fa-shell-sidebar')).toBeTruthy();
    expect(screen.getByText('nội dung').closest('.fa-shell-sheet')).toBeTruthy();
    expect(shell.style.getPropertyValue('--sidebar-w')).toBe(`${SIDEBAR_WIDTH}px`);
  });

  it('keeps room for the macOS traffic lights and a tall enough drag row', () => {
    expect(TRAFFIC_LIGHT_INSET).toBe(78);
    expect(TITLEBAR_HEIGHT).toBeGreaterThanOrEqual(38);
    expect(css).toMatch(/\.fa-shell-titlebar\s*{[^}]*padding-left:\s*78px/);
    expect(css).toMatch(/\.fa-shell-titlebar\s*{[^}]*height:\s*38px/);
  });

  it('lets the window effect show through: transparent page, opaque sheet', () => {
    expect(css).toMatch(/html,\s*body,\s*#root\s*{[^}]*background:\s*transparent/);
    expect(css).toMatch(/\.fa-shell-sidebar\s*{[^}]*background:\s*transparent/);
    expect(css).toMatch(/\.fa-shell-sheet\s*{[^}]*background:\s*var\(--sheet\)/);
    expect(css).toMatch(/\.fa-shell\s*{[^}]*grid-template-columns:\s*var\(--sidebar-w\)\s+minmax\(0,\s*1fr\)/);
  });

  it('collapses to a 56px rail that stays in the layout, animated at 220ms', () => {
    render(<WindowShell collapsed sidebar={<nav>x</nav>}><p>y</p></WindowShell>);
    expect(RAIL_WIDTH).toBe(56);
    expect(screen.getByTestId('window-shell').style.getPropertyValue('--sidebar-w')).toBe('56px');
    expect(css).toMatch(/\.fa-shell\s*{[^}]*transition:\s*grid-template-columns\s+220ms/);
  });
});
