import { useCallback, useEffect, useRef, useState } from 'react';

export type GroupKey = 'specimen' | 'metrics' | 'class' | 'meta';
export const GROUPS: readonly GroupKey[] = ['specimen', 'metrics', 'class', 'meta'];
export type RowConfig = { collapsed: GroupKey[]; hidden: string[] };
export const ROW_CONFIG_KEY = 'fa.compare.rows';
const EMPTY: RowConfig = { collapsed: [], hidden: [] };

const strs = (x: unknown): string[] => (Array.isArray(x) ? x.filter((v): v is string => typeof v === 'string') : []);

/** Đọc cấu hình hàng từ localStorage; hỏng/không có/bị chặn → mặc định (không ném lỗi). */
export function loadRowConfig(): RowConfig {
  try {
    const raw = localStorage.getItem(ROW_CONFIG_KEY);
    if (!raw) return EMPTY;
    const o = JSON.parse(raw) as { collapsed?: unknown; hidden?: unknown };
    return { collapsed: strs(o.collapsed).filter((g): g is GroupKey => (GROUPS as readonly string[]).includes(g)), hidden: strs(o.hidden) };
  } catch { return EMPTY; }
}
export function saveRowConfig(c: RowConfig): void {
  try { localStorage.setItem(ROW_CONFIG_KEY, JSON.stringify(c)); } catch { /* Safari riêng tư / chặn: bỏ qua, cấu hình chỉ sống trong phiên */ }
}

export function useRowConfig() {
  const [config, setConfig] = useState<RowConfig>(loadRowConfig);
  const update = useCallback((fn: (c: RowConfig) => RowConfig) => setConfig(fn), []);
  // Ghi ra localStorage ngoài updater của setState (updater phải thuần; StrictMode gọi nó hai lần).
  const first = useRef(true);
  useEffect(() => { if (first.current) { first.current = false; return; } saveRowConfig(config); }, [config]);
  return {
    config,
    toggleGroup: (g: GroupKey) => update(c => ({ ...c, collapsed: c.collapsed.includes(g) ? c.collapsed.filter(x => x !== g) : [...c.collapsed, g] })),
    hideRow: (id: string) => update(c => (c.hidden.includes(id) ? c : { ...c, hidden: [...c.hidden, id] })),
    showRow: (id: string) => update(c => ({ ...c, hidden: c.hidden.filter(x => x !== id) })),
  };
}
