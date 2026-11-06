import { act, fireEvent, render, screen, within } from '@testing-library/react';
import type { FontFaceLoader } from '../../../features/library-grid/fontFaceLoader';
import { describe, expect, it, vi } from 'vitest';
import type { FamilyStyleDto } from '../../../api';
import { StyleDropdown, orderStyles, styleLabel } from '../StyleDropdown';

const st = (sub: string, weight: number | null, italic = false, h = sub): FamilyStyleDto => ({ face: { hashHex: h, faceIndex: 0 }, subfamily: sub, weight, italic, instance: null });
const STYLES = [st('Bold Italic', 700, true), st('Regular', 400), st('Bold', 700), st('Light', 300), st('Italic', 400, true), st('Light Italic', 300, true)];

describe('orderStyles', () => {
  it('romans by weight, then italics by weight', () => {
    const { romans, italics } = orderStyles(STYLES);
    expect(romans.map(styleLabel)).toEqual(['Light', 'Regular', 'Bold']);
    expect(italics.map(styleLabel)).toEqual(['Light Italic', 'Italic', 'Bold Italic']);
  });
  it('labels a variable instance by its instance name and defaults weight to 400', () => {
    const vf: FamilyStyleDto = { face: { hashHex: 'v', faceIndex: 0 }, subfamily: null, weight: null, italic: false, instance: { name: 'SemiBold', coords: [{ tag: 'wght', value: 600 }] } };
    expect(styleLabel(vf)).toBe('SemiBold');
    expect(orderStyles([vf, st('Regular', 400)]).romans.map(styleLabel)).toEqual(['Regular', 'SemiBold']);
  });
});

function mount(over: Partial<React.ComponentProps<typeof StyleDropdown>> = {}) {
  const onChange = vi.fn();
  const r = render(<div><StyleDropdown styles={STYLES} value="Regular" onChange={onChange} {...over} /><button>outside</button></div>);
  return { onChange, ...r };
}
const trigger = () => screen.getByRole('button', { name: 'Kiểu chữ' });

describe('StyleDropdown', () => {
  it('shows the current style, the count, and is closed at first', () => {
    mount();
    expect(trigger().textContent).toContain('Regular');
    expect(trigger().textContent).toContain('6 kiểu');
    expect(trigger().getAttribute('aria-haspopup')).toBe('listbox');
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('lists options with a ✓ on the current one, the weight on the right, and a separator before italics', () => {
    mount();
    fireEvent.click(trigger());
    const opts = screen.getAllByRole('option');
    expect(opts.map(o => o.getAttribute('aria-label'))).toEqual(['Light', 'Regular', 'Bold', 'Light Italic', 'Italic', 'Bold Italic']);
    const sel = opts.find(o => o.getAttribute('aria-selected') === 'true')!;
    expect(sel.getAttribute('aria-label')).toBe('Regular');
    expect(within(sel).getByText('✓')).toBeTruthy();
    expect(within(opts[2]).getByText('700')).toBeTruthy();
    expect(screen.getByRole('listbox').querySelectorAll('[role=separator]')).toHaveLength(1);
  });

  it('click chooses and closes', () => {
    const { onChange } = mount();
    fireEvent.click(trigger());
    fireEvent.click(screen.getByRole('option', { name: 'Bold' }));
    expect(onChange).toHaveBeenCalledWith('Bold');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('keyboard: ArrowDown opens on the current item, ↑↓ move (wrapping not required), Enter chooses, Esc closes', () => {
    const { onChange } = mount();
    fireEvent.keyDown(trigger(), { key: 'ArrowDown' });
    const list = screen.getByRole('listbox');
    expect(list.getAttribute('aria-activedescendant')).toBe(screen.getByRole('option', { name: 'Regular' }).id);
    fireEvent.keyDown(list, { key: 'ArrowDown' });
    expect(list.getAttribute('aria-activedescendant')).toBe(screen.getByRole('option', { name: 'Bold' }).id);
    fireEvent.keyDown(list, { key: 'ArrowDown' }); // sang nhóm nghiêng, bỏ qua vạch chia
    expect(list.getAttribute('aria-activedescendant')).toBe(screen.getByRole('option', { name: 'Light Italic' }).id);
    fireEvent.keyDown(list, { key: 'ArrowUp' });
    fireEvent.keyDown(list, { key: 'ArrowUp' });
    fireEvent.keyDown(list, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledWith('Regular');
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(document.activeElement).toBe(trigger());
  });

  it('Esc closes without choosing; Home/End jump', () => {
    const { onChange } = mount();
    fireEvent.click(trigger());
    const list = screen.getByRole('listbox');
    fireEvent.keyDown(list, { key: 'End' });
    expect(list.getAttribute('aria-activedescendant')).toBe(screen.getByRole('option', { name: 'Bold Italic' }).id);
    fireEvent.keyDown(list, { key: 'Home' });
    expect(list.getAttribute('aria-activedescendant')).toBe(screen.getByRole('option', { name: 'Light' }).id);
    fireEvent.keyDown(list, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });

  it('clicking outside closes', () => {
    mount();
    fireEvent.click(trigger());
    fireEvent.mouseDown(screen.getByText('outside'));
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('a single style disables the dropdown but still shows its name', () => {
    mount({ styles: [st('Regular', 400)] });
    expect((trigger() as HTMLButtonElement).disabled).toBe(true);
    expect(trigger().textContent).toContain('Regular');
    fireEvent.click(trigger());
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('renders each option in its own font once loaded, and keeps the UI font + weight/italic until then', async () => {
    const pending = new Map<string, (f: string | null) => void>();
    const loader = {
      acquire: vi.fn((r: { hashHex: string }) => new Promise<string | null>(res => pending.set(r.hashHex, res))),
      release: vi.fn(),
    } as unknown as FontFaceLoader;
    mount({ loader });
    fireEvent.click(trigger());
    const bold = () => screen.getByRole('option', { name: 'Bold' }).querySelector('.lbl') as HTMLElement;
    // chưa nạp xong: font giao diện, kiểu CSS theo style
    expect(bold().style.fontFamily).toBe('');
    expect(screen.getByRole('option', { name: 'Bold' }).style.fontWeight).toBe('700');
    expect(screen.getByRole('option', { name: 'Light Italic' }).style.fontStyle).toBe('italic');
    await act(async () => { pending.get('Bold')!('fa-bold'); });
    expect(bold().style.fontFamily).toContain('fa-bold');
    expect(bold().style.fontFamily).toContain('var(--font-ui)'); // fallback cho glyph thiếu
    expect(bold().style.fontWeight).toBe('400'); // không in đậm giả trên font đã là Bold
    expect(bold().dataset.ownFont).toBe('fa-bold');
  });
});
