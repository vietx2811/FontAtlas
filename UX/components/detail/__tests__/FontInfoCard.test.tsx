import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { FaceDetailDto } from '../../../api';
import { FontInfoCard } from '../FontInfoCard';

async function detail(over: Partial<FaceDetailDto['metadata']> = {}): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({ limit: 1 })).items[0];
  const d = await api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
  return { ...d, metadata: { ...d.metadata, ...over } };
}

describe('FontInfoCard', () => {
  it('shows only Thiết kế and Giấy phép by default', async () => {
    render(<FontInfoCard detail={await detail()} expanded={false} onToggle={() => {}} onReveal={() => {}} />);
    expect(screen.getByText('Thiết kế')).toBeTruthy();
    expect(screen.getByText('Giấy phép')).toBeTruthy();
    for (const k of ['Hãng', 'Phiên bản', 'Bản quyền', 'Tệp']) expect(screen.queryByText(k)).toBeNull();
    expect(screen.getByRole('button', { name: /Hãng · Phiên bản · Bản quyền · Tệp/ }).getAttribute('aria-expanded')).toBe('false');
  });

  it('expands to the rest, with the file path in mono and truncated by CSS (full path in title)', async () => {
    const d = await detail();
    const onToggle = vi.fn();
    render(<FontInfoCard detail={d} expanded onToggle={onToggle} onReveal={() => {}} />);
    for (const k of ['Hãng', 'Phiên bản', 'Bản quyền', 'Tệp']) expect(screen.getByText(k)).toBeTruthy();
    const path = screen.getByText(d.file.path!);
    expect(path.className).toContain('mono');
    expect(path.getAttribute('title')).toBe(d.file.path);
    fireEvent.click(screen.getByRole('button', { name: 'Thu gọn' }));
    expect(onToggle).toHaveBeenCalled();
  });

  it('hides rows whose value is empty instead of inventing one', async () => {
    render(<FontInfoCard detail={await detail({ designer: null, copyright: '', manufacturer: null })} expanded onToggle={() => {}} onReveal={() => {}} />);
    for (const k of ['Thiết kế', 'Hãng', 'Bản quyền']) expect(screen.queryByText(k)).toBeNull();
    expect(screen.getByText('Giấy phép')).toBeTruthy();
    expect(screen.getByText('Phiên bản')).toBeTruthy();
  });

  it('right-click on the file row offers "Hiện trong Finder"', async () => {
    const onReveal = vi.fn();
    const d = await detail();
    render(<FontInfoCard detail={d} expanded onToggle={() => {}} onReveal={onReveal} />);
    fireEvent.contextMenu(screen.getByText(d.file.path!));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Hiện trong Finder' }));
    expect(onReveal).toHaveBeenCalled();
  });

  it('renders nothing while there is no detail', () => {
    const { container } = render(<FontInfoCard detail={null} expanded={false} onToggle={() => {}} onReveal={() => {}} />);
    expect(container.textContent).toBe('');
  });
});
