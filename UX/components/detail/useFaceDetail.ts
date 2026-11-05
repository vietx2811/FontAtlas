import { useCallback, useEffect, useState } from 'react';
import type { Api, FaceDetailDto, FaceKeyDto, FamilyStyleDto } from '../../api';
import { parseFaceKey, useUiStore } from '../../store/ui';
import { styleLabel } from './StyleDropdown';

type State = { detail: FaceDetailDto | null; styles: FamilyStyleDto[]; error: string | null; loading: boolean };

/**
 * Face đang chọn → chi tiết + các kiểu cùng family. Kiểu đã chọn (store.style) quyết định face thực sự được đọc.
 * Mọi tab đọc `detail`, nên đổi kiểu là mọi tab đổi theo.
 */
export function useFaceDetail(api: Api): State & { retry(): void; face: FaceKeyDto | null } {
  const selectedId = useUiStore(s => s.selectedId);
  const style = useUiStore(s => s.style);
  const [state, setState] = useState<State>({ detail: null, styles: [], error: null, loading: false });
  const [attempt, setAttempt] = useState(0);
  const [family, setFamily] = useState<{ id: string; family: string; styles: FamilyStyleDto[] } | null>(null);

  // Bước 1: family + danh sách kiểu của face được chọn trong lưới.
  useEffect(() => {
    if (!selectedId) { setFamily(null); setState({ detail: null, styles: [], error: null, loading: false }); return; }
    let alive = true;
    setState(s => ({ ...s, error: null, loading: true }));
    (async () => {
      const d = await api.getFace(parseFaceKey(selectedId), 'vi');
      const styles = await api.getFamilyStyles(d.row.family);
      if (alive) setFamily({ id: selectedId, family: d.row.family, styles });
    })().catch(e => { if (alive) setState(s => ({ ...s, error: e?.message ?? String(e), loading: false })); });
    return () => { alive = false; };
  }, [api, selectedId, attempt]);

  // Bước 2: face của kiểu đang chọn (mặc định: chính face được chọn).
  const picked = family && family.id === selectedId ? family.styles.find(s => styleLabel(s) === style) : undefined;
  const face = selectedId ? (picked?.face ?? parseFaceKey(selectedId)) : null;
  const faceId = face ? `${face.hashHex}:${face.faceIndex}` : null;
  useEffect(() => {
    if (!face || !family || family.id !== selectedId) return;
    let alive = true;
    api.getFace(face, 'vi').then(
      d => { if (alive) setState({ detail: d, styles: family.styles, error: null, loading: false }); },
      e => { if (alive) setState(s => ({ ...s, error: e?.message ?? String(e), loading: false })); },
    );
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, family, faceId]);

  const retry = useCallback(() => setAttempt(n => n + 1), []);
  return { ...state, retry, face };
}
