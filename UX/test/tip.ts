import { act, fireEvent, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { TOOLTIP_DELAY_MS } from '../components/ui/Tooltip';

/** Rê chuột vào phần tử, chờ qua độ trễ của Tooltip, trả chữ đang hiện (null nếu không có), rồi rời đi. */
export async function tipOf(el: Element): Promise<string | null> {
  fireEvent.pointerEnter(el);
  await act(async () => { if (vi.isFakeTimers()) await vi.advanceTimersByTimeAsync(TOOLTIP_DELAY_MS + 40); else await new Promise(r => setTimeout(r, TOOLTIP_DELAY_MS + 40)); });
  const text = screen.queryByRole('tooltip')?.textContent ?? null;
  fireEvent.pointerLeave(el);
  return text;
}
