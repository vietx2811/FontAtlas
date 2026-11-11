import { useEffect, useRef, useState } from 'react';
import type { FamilyGroup } from './groupLayout';
import type { GridQuery } from './types';

/**
 * Chỉ mục nhóm Family của truy vấn hiện tại (R3). Chỉ nạp khi `enabled`; truy vấn đổi → bỏ kết quả cũ ngay (không dựng bố cục
 * bằng nhóm của truy vấn khác). Lỗi → `error`, lưới rơi về dạng phẳng.
 */
export function useFamilyGroups(api: { familyGroups?(q: GridQuery): Promise<FamilyGroup[]> }, query: GridQuery, enabled: boolean, rev = 0): { groups: FamilyGroup[] | null; error: string | null; queryKey: string } {
  const [state, setState] = useState<{ key: string; groups: FamilyGroup[] | null; error: string | null }>({ key: '', groups: null, error: null });
  const queryKey = JSON.stringify(query);
  const key = enabled ? `${rev}\0${JSON.stringify(query)}` : '';
  const seq = useRef(0);
  const queryRef = useRef(query); queryRef.current = query;
  useEffect(() => {
    const my = ++seq.current;
    if (!enabled || !api.familyGroups) { setState(s => (s.key === '' && s.groups === null ? s : { key: '', groups: null, error: null })); return; }
    api.familyGroups(queryRef.current).then(
      groups => { if (seq.current === my) setState({ key, groups, error: null }); },
      e => { if (seq.current === my) setState({ key, groups: null, error: (e as Error)?.message ?? String(e) }); },
    );
    return () => { seq.current++; };
  }, [api, enabled, key]); // eslint-disable-line react-hooks/exhaustive-deps
  return state.key === key ? { groups: state.groups, error: state.error, queryKey } : { groups: null, error: null, queryKey: '' };
}
