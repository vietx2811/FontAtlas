// Mô phỏng dịch vụ kích hoạt của Plan 7 (hợp đồng 6a): lý do thủ công/theo dự án, xung đột tên, tự gỡ khi rời dự án.
import type { FaceRow } from '../features/library-grid/types';
import type {
  ActivationEvent, ActivationReportDto, ActivationStateDto, AutoDeactivatedDto, FaceKeyDto, FileOutcomeDto, FileResultDto, ReasonDto,
} from './types';
import type { ActivateOptions, DeactivateOptions } from './index';

/** Họ "Roboto" giả lập trùng tên với font hệ thống (cần xác nhận); "Lobster" giả lập file đã đổi trên đĩa. */
export const CONFLICT_FAMILY = 'Roboto';
/** Họ "Oswald" trùng với font của RightFont (được phép kích hoạt tiếp). */
export const OTHER_MANAGER_FAMILY = 'Oswald';
export const CHANGED_FAMILY = 'Lobster';

type Entry = { reasons: ReasonDto[]; scope: 'session' | 'persistent' };
type Deps = { rowOf(k: FaceKeyDto): FaceRow | undefined; collectionFaces(id: number): FaceKeyDto[] };

const same = (a: ReasonDto, b: ReasonDto) => a.kind === b.kind && (a.kind !== 'collection' || (b.kind === 'collection' && a.collectionId === b.collectionId));

export function createActivationSim(d: Deps) {
  const state = new Map<string, Entry>();
  const listeners = new Set<(e: ActivationEvent) => void>();
  const emit = (e: ActivationEvent) => listeners.forEach(l => l(e));
  const hashes = (faces: FaceKeyDto[]) => [...new Map(faces.map(f => [f.hashHex, f])).values()];
  const result = (f: FaceKeyDto, outcome: FileOutcomeDto): FileResultDto => ({ hashHex: f.hashHex, path: d.rowOf(f)?.path ?? null, requestedFaces: [f.faceIndex], extraFaces: [], outcome });
  const report = (results: FileResultDto[]): ActivationReportDto => ({
    results,
    changed: results.filter(r => r.outcome.kind === 'activated' || r.outcome.kind === 'deactivated').length,
    needsConfirmation: results.filter(r => r.outcome.kind === 'needs_confirmation').length,
    failed: results.filter(r => r.outcome.kind === 'failed').length,
  });
  const add = (h: string, r: ReasonDto, scope: Entry['scope'] = 'session') => {
    const e = state.get(h) ?? { reasons: [], scope };
    if (!e.reasons.some(x => same(x, r))) e.reasons.push(r);
    state.set(h, e);
  };
  const conflictOf = (row: FaceRow | undefined) => {
    const owner = row?.family === CONFLICT_FAMILY ? 'system' as const : row?.family === OTHER_MANAGER_FAMILY ? 'right_font' as const : null;
    return row && owner ? [{ postscriptName: `${row.family}-${row.subfamily ?? 'Regular'}`, existingPath: owner === 'system' ? `/Library/Fonts/${row.family}.ttf` : `~/Library/RightFont/${row.family}.ttf`, owner }] : [];
  };

  const activateOne = (f: FaceKeyDto, reason: ReasonDto, o: ActivateOptions): FileOutcomeDto => {
    const row = d.rowOf(f);
    if (row?.family === CHANGED_FAMILY) return { kind: 'failed', code: 'file_changed', message: 'file changed on disk since scan' };
    const conflicts = conflictOf(row);
    if (conflicts.length && !o.proceed) return { kind: 'needs_confirmation', conflicts };
    const had = state.get(f.hashHex)?.reasons.length;
    add(f.hashHex, reason, o.scope ?? 'session');
    return had ? { kind: 'already_active' } : { kind: 'activated' };
  };
  const finish = (rs: FileResultDto[], onProgress?: ActivateOptions['onProgress']) => {
    rs.forEach((r, i) => onProgress?.({ done: i + 1, total: rs.length, hashHex: r.hashHex }));
    emit({ type: 'changed', hashes: rs.map(r => r.hashHex) });
    return report(rs);
  };
  /** Gỡ một lý do; hết lý do thì tắt hẳn. Trả cause nếu font bị tự gỡ. */
  const drop = (h: string, r: ReasonDto): boolean => {
    const e = state.get(h);
    if (!e) return false;
    e.reasons = e.reasons.filter(x => !same(x, r));
    if (e.reasons.length === 0) { state.delete(h); return true; }
    return false;
  };
  const reconcile = (gone: Array<[FaceKeyDto, ReasonDto]>, cause: string, failed: Set<string> = new Set()) => {
    const autoDeactivated: AutoDeactivatedDto[] = []; const autoDeactivateFailed: AutoDeactivatedDto[] = [];
    for (const [f, r] of gone) {
      if (!drop(f.hashHex, r)) continue;
      const item = { hashHex: f.hashHex, faces: [f.faceIndex], removedReasons: [r], cause };
      if (failed.has(f.hashHex)) { state.set(f.hashHex, { reasons: [], scope: 'session' }); autoDeactivateFailed.push(item); } else autoDeactivated.push(item);
    }
    emit({ type: 'reconciled', report: { autoDeactivated, autoDeactivateFailed, reregistered: 0, moved: 0, missing: 0, notDownloaded: 0, cleaned: 0, failed: autoDeactivateFailed.length, results: [] } });
  };

  return {
    /** Dành cho mock: ép "hệ điều hành từ chối gỡ" cho các hash này ở lần tự gỡ kế tiếp. */
    failNextAutoDeactivate: new Set<string>(),
    async activationCaps() { return { method: 'fake', supported: true }; },
    async activationState(): Promise<ActivationStateDto[]> {
      return [...state].map(([hashHex, e]) => ({
        hashHex, faces: [0], state: 'active', scope: e.scope, status: 'registered', reasons: e.reasons.map(r => ({ ...r })), activePath: null, conflictPs: null, lastError: null,
      }));
    },
    async activationPreflight(faces: FaceKeyDto[]) {
      return report(hashes(faces).map(f => result(f, conflictOf(d.rowOf(f)).length ? { kind: 'needs_confirmation', conflicts: conflictOf(d.rowOf(f)) } : { kind: 'ready' })));
    },
    async activationActivate(faces: FaceKeyDto[], o: ActivateOptions = {}) {
      return finish(hashes(faces).map(f => result(f, activateOne(f, { kind: 'manual' }, o))), o.onProgress);
    },
    async activationDeactivate(faces: FaceKeyDto[], o: DeactivateOptions = {}) {
      return finish(hashes(faces).map(f => {
        const e = state.get(f.hashHex);
        if (!e) return result(f, { kind: 'already_inactive' });
        const cols = e.reasons.flatMap(r => (r.kind === 'collection' ? [r.collectionId] : []));
        if (cols.length && !o.force) return result(f, { kind: 'held_by_collections', collectionIds: cols });
        state.delete(f.hashHex);
        return result(f, { kind: 'deactivated' });
      }), o.onProgress);
    },
    async activationActivateCollection(id: number, o: ActivateOptions = {}) {
      return finish(hashes(d.collectionFaces(id)).map(f => result(f, activateOne(f, { kind: 'collection', collectionId: id }, o))), o.onProgress);
    },
    async activationDeactivateCollection(id: number, o: Pick<DeactivateOptions, 'onProgress'> = {}) {
      return finish(hashes(d.collectionFaces(id)).map(f => {
        const e = state.get(f.hashHex);
        if (!e) return result(f, { kind: 'already_inactive' });
        const gone = drop(f.hashHex, { kind: 'collection', collectionId: id });
        if (gone) return result(f, { kind: 'deactivated' });
        const left = state.get(f.hashHex)!.reasons;
        return result(f, { kind: 'kept_active', manual: left.some(r => r.kind === 'manual'), collectionIds: left.flatMap(r => (r.kind === 'collection' ? [r.collectionId] : [])) });
      }), o.onProgress);
    },
    async activationDeactivateAll(o: Pick<DeactivateOptions, 'onProgress'> = {}) {
      const all = [...state.keys()].map(h => ({ hashHex: h, faceIndex: 0 }));
      state.clear();
      return finish(all.map(f => result(f, { kind: 'deactivated' })), o.onProgress);
    },
    async activationSetKeep() {},
    async activationSetScope(faces: FaceKeyDto[], scope: 'session' | 'persistent') {
      const changed = hashes(faces).filter(f => { const e = state.get(f.hashHex); if (!e || e.scope === scope) return false; e.scope = scope; return true; });
      return finish(changed.map(f => result(f, { kind: 'activated' })));
    },
    async activationSubscribe(l: (e: ActivationEvent) => void) { listeners.add(l); return () => { listeners.delete(l); }; },
    /** Gọi khi dự án mất mặt: `removed` = (face, dự án) bị gỡ. */
    collectionChanged(removed: Array<[FaceKeyDto, number]>, cause: 'collection_deleted' | 'left_collection') {
      reconcile(removed.map(([f, id]) => [f, { kind: 'collection', collectionId: id }]), cause, this.failNextAutoDeactivate);
      this.failNextAutoDeactivate.clear();
    },
    /** Font thêm vào dự án đang được kích hoạt thì cũng được bật. */
    collectionGained(f: FaceKeyDto, id: number) { if ([...state.values()].some(e => e.reasons.some(r => r.kind === 'collection' && r.collectionId === id))) add(f.hashHex, { kind: 'collection', collectionId: id }); },
  };
}
