import { CmdError } from './types';
import type { AutoAppDto, AutoLogDto, AutoStatusDto, AutoTargetDto, OtherManagerDto } from './types';

/** Bật font tự động (mock, gương services/auto.rs): 6 app cố định thứ tự, cài/gỡ chỉ đổi cờ trong bộ nhớ. */
export type MockAutoOptions = { isolated?: boolean; keyPresent?: boolean; enabled?: boolean; others?: OtherManagerDto[]; running?: string[]; installed?: string[]; log?: AutoLogDto[] };
const APPS: Array<Pick<AutoAppDto, 'id' | 'label' | 'method' | 'evidence' | 'installable'>> = [
  { id: 'photoshop', label: 'Photoshop', method: 'font_request', evidence: 'measured', installable: false },
  { id: 'aftereffects', label: 'After Effects', method: 'font_request', evidence: 'untested', installable: false },
  { id: 'illustrator', label: 'Illustrator', method: 'cep_panel', evidence: 'partial', installable: true },
  { id: 'indesign', label: 'InDesign', method: 'startup_script', evidence: 'partial', installable: true },
  { id: 'incopy', label: 'InCopy', method: 'startup_script', evidence: 'untested', installable: true },
  { id: 'affinity', label: 'Affinity', method: 'font_request', evidence: 'untested', installable: false },
];
const TARGET_APPS: Record<AutoTargetDto, string[]> = { illustrator: ['illustrator'], indesign: ['indesign', 'incopy'] };

export function createMockAuto(o: MockAutoOptions = {}) {
  let enabled = o.enabled ?? true;
  const installed = new Set(o.installed ?? []);
  const running = new Set(o.running ?? []);
  const others: OtherManagerDto[] = o.others ?? [{ name: 'RightFont', installed: false, running: false }, { name: 'FontBase', installed: false, running: false }, { name: 'Typeface', installed: false, running: false }, { name: 'Suitcase Fusion', installed: false, running: false }, { name: 'FontExplorer X', installed: false, running: false }];
  const log: AutoLogDto[] = o.log ?? [];
  let key = o.keyPresent ?? installed.has('illustrator'); // khoá được tạo lần cài Illustrator đầu tiên; gỡ panel KHÔNG xoá khoá
  const isolated = o.isolated ?? false;
  const refuse = () => { if (isolated) throw new CmdError('isolated_build', 'isolated build'); };
  const status = (): AutoStatusDto => ({
    enabled, brokerRunning: enabled, fontRequestRunning: enabled && !isolated, signingKeyPresent: key, isolatedBuild: isolated,
    apps: APPS.map(a => {
      const inst = a.installable ? installed.has(a.id) : true;
      const state: AutoAppDto['state'] = !enabled ? 'off' : a.installable && !inst ? 'needs_install' : 'ready';
      const last = log.find(l => l.app === a.id);
      return { ...a, state, appInstalled: true, appRunning: running.has(a.id), installed: a.installable ? inst : false, restartNeeded: a.installable && inst && running.has(a.id), detail: null, lastActivityMs: last ? last.atMs : null };
    }),
    otherManagers: others,
  });
  return {
    async autoStatus() { return status(); },
    async autoSetEnabled(on: boolean) { enabled = on; return status(); },
    async autoInstall(target: AutoTargetDto) { refuse(); if (target === 'illustrator') key = true; for (const a of TARGET_APPS[target]) installed.add(a); return status(); },
    async autoForgetKey() { refuse(); if (installed.has('illustrator')) throw new CmdError('io', 'panel still installed'); key = false; return status(); },
    async autoRemove(target: AutoTargetDto) { refuse(); for (const a of TARGET_APPS[target]) installed.delete(a); return status(); },
    async autoLog(limit = 50) { return log.slice(0, Math.max(1, Math.min(500, limit))); },
    /** Chỉ cho test/dev: thêm dòng nhật ký và đổi app đang chạy. */
    __push(l: Omit<AutoLogDto, 'id'>) { log.unshift({ ...l, id: BigInt(log.length + 1) }); },
    __setRunning(app: string, on: boolean) { if (on) running.add(app); else running.delete(app); },
    __setManager(name: string, patch: Partial<OtherManagerDto>) { const m = others.find(x => x.name === name); if (m) Object.assign(m, patch); },
  };
}
