import { useUiStore } from '../../store/ui';
import './composer.css';

export const COMPOSER_INSET = 120; // khoảng chừa ở đáy lưới để thẻ cuối không bị ô nổi che
const PRESETS: Array<[label: string, text: string, name: string]> = [['Aa', 'Aa', 'Aa'], ['Ặẫ', 'Ặẫỡự', 'Dấu chồng'], ['12', '0123456789', 'Số']];

export function SampleComposer(): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const size = useUiStore(s => s.sampleSize);
  const setSample = useUiStore(s => s.setSample);
  const setSize = useUiStore(s => s.setSampleSize);
  return (
    <div className="fa-composer" role="group" aria-label="Chữ mẫu">
      <input type="text" value={sample} autoComplete="off" aria-label="Chữ mẫu" onChange={e => setSample(e.target.value)} />
      <div className="fa-presets">
        {PRESETS.map(([label, text, name]) => (
          <button key={text} type="button" title={name} aria-label={name} onClick={() => setSample(text)}>{label}</button>
        ))}
      </div>
      <div className="fa-size" title="Cỡ chữ mẫu">
        <span className="a1" aria-hidden="true">A</span>
        <input type="range" min={18} max={72} value={size} aria-label="Cỡ chữ mẫu" onChange={e => setSize(Number(e.target.value))} />
        <span className="a2" aria-hidden="true">A</span>
      </div>
    </div>
  );
}
