import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = resolve(__dirname, '../../..');
const conf = JSON.parse(readFileSync(resolve(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const main = conf.app.windows.find((w: { label: string }) => w.label === 'main');

describe('cửa sổ main: vibrancy sidebar của macOS (tauri 2.9.5)', () => {
  it('transparent, overlay title bar, sidebar effect follows window active state', () => {
    expect(main.transparent).toBe(true);
    expect(main.titleBarStyle).toBe('Overlay');
    expect(main.windowEffects).toEqual({ effects: ['sidebar'], state: 'followsWindowActiveState' });
    expect(main.minWidth).toBe(1000);
    expect(main.minHeight).toBe(700);
  });

  it('enables the macOS private API in config and as a tauri cargo feature (must match)', () => {
    expect(conf.app.macOSPrivateApi).toBe(true);
    const cargo = readFileSync(resolve(root, 'src-tauri/Cargo.toml'), 'utf8');
    expect(cargo).toMatch(/^tauri = \{ version = "2\.9\.5", features = \[[^\]]*"macos-private-api"[^\]]*\] \}$/m);
  });
});
