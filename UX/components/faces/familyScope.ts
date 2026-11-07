import type { Api } from '../../api';
import { faceKey, type FamilyScope } from '../../store/ui';

export const FAMILY_MAX = 2000;

/** Toàn bộ kiểu của một family làm phạm vi lưới (giống nút "Xem family" ở tab Tương tự). */
export async function fetchFamilyScope(api: Api, family: string): Promise<FamilyScope> {
  const styles = await api.getFamilyStyles(family);
  const faces = [...new Map(styles.map(s => [faceKey(s.face), s.face])).values()];
  if (!faces.length) throw new Error('Family chưa có kiểu font khả dụng.');
  if (faces.length > FAMILY_MAX) throw new Error('Family có hơn 2.000 kiểu; hãy mở font đại diện.');
  return { name: family, faces };
}
