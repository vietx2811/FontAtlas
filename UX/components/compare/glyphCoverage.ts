import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';

type Seg = new (l?: string, o?: object) => { segment(s: string): Iterable<{ segment: string }> };
const SEGMENTER = (() => { try { const S = (Intl as { Segmenter?: Seg }).Segmenter; return S ? new S('vi', { granularity: 'grapheme' }) : null; } catch { return null; } })();
/** Tách theo grapheme (chữ + dấu đi cùng nhau, kể cả dạng NFD); không có Segmenter thì theo ký tự sau NFC. */
export function graphemes(s: string): string[] {
  return SEGMENTER ? [...SEGMENTER.segment(s)].map(x => x.segment) : [...s.normalize('NFC')];
}

const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const cache = new Map<string, boolean>(); // `${face}\u0000${grapheme}` → thiếu?
export const clearCoverageCache = (): void => cache.clear();

/**
 * Grapheme của `text` mà font KHÔNG có glyph, theo cmap thật của file font (backend `missing_chars`).
 * Không đoán bằng hình vẽ canvas (cách đó báo nhầm khi font trùng font hệ thống). null = không hỏi được.
 */
export async function detectMissing(api: Api, face: FaceKeyDto, text: string): Promise<Set<string> | null> {
  const gs = [...new Set(graphemes(text))].filter(g => g.trim());
  if (gs.length === 0) return new Set();
  const id = keyOf(face);
  const todo = gs.filter(g => !cache.has(`${id}\u0000${g}`));
  if (todo.length) {
    let missing: string[];
    try { missing = await api.missingChars(face, todo); } catch { return null; }
    const set = new Set(missing);
    for (const g of todo) cache.set(`${id}\u0000${g}`, set.has(g));
  }
  return new Set(gs.filter(g => cache.get(`${id}\u0000${g}`)));
}

/** Điểm gọi (đổi được trong test). */
export const coverageProbe = { detect: detectMissing };

const NONE: ReadonlySet<string> = new Set();
export const MISSING_DEBOUNCE_MS = 150;
/**
 * Grapheme của `text` mà face không có. Rỗng khi chưa hỏi lần nào / không hỏi được.
 * Gõ liên tục: GIỮ kết quả cũ của cùng face trong lúc hỏi lại (không nháy chữ xám / NoteMark), hỏi lại sau ~150 ms, cache theo (face, grapheme).
 */
export function useMissingChars(api: Api, face: FaceKeyDto, text: string): ReadonlySet<string> {
  const id = keyOf(face);
  const [state, setState] = useState<{ id: string; missing: ReadonlySet<string> } | null>(null);
  const had = useRef<string | null>(null);
  useEffect(() => {
    let alive = true;
    const run = () => { void coverageProbe.detect(api, face, text).then(m => { if (alive) { had.current = id; setState({ id, missing: m ?? NONE }); } }); };
    const h = setTimeout(run, had.current === id ? MISSING_DEBOUNCE_MS : 0);
    return () => { alive = false; clearTimeout(h); };
  }, [api, id, text]); // eslint-disable-line react-hooks/exhaustive-deps
  return state && state.id === id ? state.missing : NONE;
}

/** Ghi chú chữ cho tooltip/PNG: "ấ ợ …" (tối đa `max` ký tự). */
export function missingList(missing: ReadonlySet<string>, max = 12): string {
  const a = [...missing];
  return a.slice(0, max).join(' ') + (a.length > max ? ' …' : '');
}
