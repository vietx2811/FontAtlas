// Hợp đồng IPC phía TS: fixture do Rust sinh phải qua parser của lưới (Plan 5) và khớp kiểu đã sinh.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseFacePage } from '../../features/library-grid/types';
import type { CommandName, TauriCommandMap } from '../commands';

const dir = join(__dirname, '..');
const fx = (n: string) => JSON.parse(readFileSync(join(dir, 'fixtures', `${n}.json`), 'utf8'));
const typeText = (n: string) => readFileSync(join(dir, `${n}.ts`), 'utf8');

describe('bindings contract', () => {
  it('face_page qua parser của lưới và không thừa khoá', () => {
    const page = parseFacePage(fx('face_page'));
    expect(page.items.length).toBe(2);
    const keys = Object.keys(fx('face_page').items[0]).sort();
    expect(keys).toHaveLength(19);
    for (const k of keys) expect(typeText('FaceRowDto')).toContain(`${k}:`);
  });

  it('mọi fixture là JSON hợp lệ và có đủ tên trong bản đồ lệnh', () => {
    const files = readdirSync(join(dir, 'fixtures')).filter(f => f.endsWith('.json'));
    expect(files.length).toBeGreaterThanOrEqual(19);
    for (const f of files) expect(() => JSON.parse(readFileSync(join(dir, 'fixtures', f), 'utf8'))).not.toThrow();
    const map = readFileSync(join(dir, 'commands.ts'), 'utf8');
    for (const c of ['query_faces', 'activation_activate', 'model_download', 'preview_info', 'get_face', 'set_setting'] satisfies CommandName[]) {
      expect(map).toContain(`  ${c}: {`);
    }
  });

  it('kiểu lệnh dùng được như bản đồ Plan 5', () => {
    type Q = TauriCommandMap['query_faces']['args']['q'];
    const q: Pick<Q, 'text' | 'limit'> = { text: 'ro', limit: 10 };
    expect(q.limit).toBe(10);
  });

  it('fixture kích hoạt giữ thẻ kind và faces', () => {
    const rep = fx('activation_report');
    expect(rep.results.some((r: { outcome: { kind: string } }) => r.outcome.kind === 'needs_confirmation')).toBe(true);
    expect(fx('activation_state')[0].reasons[1]).toEqual({ kind: 'collection', collectionId: 3 });
    expect(fx('activation_state')[0].faces).toEqual([0]);
    expect(fx('reconcile_report').autoDeactivated[0].cause).toBe('collection_deleted');
  });

  it('sự kiện có thẻ phân biệt', () => {
    expect(fx('scan_events').map((e: { type: string }) => e.type)).toEqual(['queued', 'progress', 'facesAdded', 'finished', 'idle']);
    expect(fx('model_events')[0].type).toBe('progress');
    expect(fx('embed_events')[0].kind).toBe('finished');
  });
});
