import type { EmptyAction } from './zeroStates';
import './zero.css';

type Props = { glyph: string; text: string; action?: EmptyAction | { id: string; label: string }; onAction?(id: string): void; compact?: boolean };

/** Trạng thái trống: một chữ minh hoạ nhỏ trên đường nền, một dòng, một hành động (nếu có). */
export function ZeroState({ glyph, text, action, onAction, compact }: Props): React.JSX.Element {
  return (
    <div className={`fa-zero${compact ? ' compact' : ''}`} data-testid="zero-state">
      <svg aria-hidden="true" viewBox="0 0 96 64" width="96" height="64">
        <line x1="10" x2="86" y1="50" y2="50" stroke="var(--accent)" strokeWidth="1" />
        <line x1="10" x2="86" y1="28" y2="28" stroke="var(--border)" strokeWidth="1" strokeDasharray="3 3" />
        <text x="48" y="50" fontSize="44" fontWeight="500" fill="var(--muted)" fontFamily="var(--font-ui)" textAnchor="middle">{glyph}</text>
      </svg>
      <p>{text}</p>
      {action && <button type="button" onClick={() => onAction?.(action.id)}>{action.label}</button>}
    </div>
  );
}
