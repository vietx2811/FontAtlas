import { useRef, useState } from 'react';
import type { CaseDto, ComboDto } from '../../bindings';
import { t } from '../../i18n';
import { Menu, type MenuAnchor, type MenuEntry } from '../ui/Menu';
import { NamePrompt } from '../ui/NamePrompt';
import { Tooltip } from '../ui/Tooltip';

type Ops = { create(): void; duplicate(): void; rename(name: string): void; makeMain(): void; remove(): void; go(id: number): void };

/** `‹ Combo n/m ▾ ›` (06 §1): bước giữa các Combo của Case, menu tạo/nhân bản/đổi tên/đặt chính/xoá. `[` `]` bước, xử lý ở Desk. */
export function ComboStepper({ data, combo, ops }: { data: CaseDto; combo: ComboDto; ops: Ops }): React.JSX.Element {
  const btn = useRef<HTMLButtonElement>(null);
  const [menu, setMenu] = useState<MenuAnchor | null>(null);
  const [rename, setRename] = useState<MenuAnchor | null>(null);
  const list = data.combos; const i = list.findIndex(c => c.id === combo.id);
  const rect = () => btn.current!.getBoundingClientRect();
  const entries: MenuEntry[] = [
    { id: 'new', label: t('combo.new'), onSelect: ops.create },
    { id: 'dup', label: t('combo.duplicate'), onSelect: ops.duplicate },
    { id: 'rename', label: t('combo.rename'), onSelect: () => setRename({ rect: rect() }) },
    { id: 'main', label: t('combo.makeMain'), disabled: combo.isMain, reason: t('combo.isMain'), onSelect: ops.makeMain },
    { type: 'sep' },
    { id: 'del', label: t('combo.delete'), danger: true, disabled: list.length < 2, reason: t('combo.lastOne'), onSelect: ops.remove },
  ];
  return (
    <span className="fa-combo" data-testid="combo-stepper">
      <Tooltip label={t('combo.prev')} shortcut="["><button type="button" data-testid="combo-prev" disabled={i <= 0} onClick={() => ops.go(list[i - 1].id)}>‹</button></Tooltip>
      <button ref={btn} type="button" className="fa-combo-label" data-testid="combo-label" aria-haspopup="menu" onClick={() => setMenu({ rect: rect() })}>
        {combo.isMain ? '★ ' : ''}Combo {i + 1}/{list.length}{combo.name ? ` · ${combo.name}` : ''} ▾
      </button>
      <Tooltip label={t('combo.next')} shortcut="]"><button type="button" data-testid="combo-next" disabled={i >= list.length - 1} onClick={() => ops.go(list[i + 1].id)}>›</button></Tooltip>
      {menu && <Menu entries={entries} anchor={menu} onClose={() => setMenu(null)} label="Combo" returnFocus={btn.current} />}
      {rename && <NamePrompt label={t('combo.rename')} initial={combo.name ?? ''} submitLabel={t('combo.save')} anchor={rename} onCancel={() => setRename(null)} onSubmit={n => { setRename(null); ops.rename(n); }} />}
    </span>
  );
}
