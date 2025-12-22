import React, { useState, useEffect } from 'react';
import { X, Type, Layers, Info, Sparkles, Users, ExternalLink, Globe, RefreshCw, Loader2, Heart } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { FontData } from '../types';
import { Button } from './Button';
import { MetricsChart } from './MetricsChart';
import { ALPHABET, SAMPLE_TEXT } from '../constants';
import { useFontLoader } from '../hooks/useFontLoader';
import { PairingTab } from './PairingTab';
import { rescanFont, toggleFavorite } from '../services/library';
import { qdrantService, SimilarFont } from '../services/qdrant';
import { deskService, useDeskStore } from '../services/desk';
import { FontActionPanel } from './FontActionPanel';
import { Plus, Check, ArrowRight } from 'lucide-react';

interface DetailPanelProps {
  font: FontData | null;
  onClose: () => void;
  library: FontData[];
  onSelectFont?: (font: FontData) => void;
  onPreview?: (font: FontData) => void;
  hasPreviewPanel?: boolean; // New prop to indicate if preview panel is open
}

const PREVIEW_SIZES = [
  { label: 'Display', size: '72px' },
  { label: 'Headline', size: '48px' },
  { label: 'Title', size: '36px' },
  { label: 'Body', size: '18px' },
  { label: 'Caption', size: '14px' }
];

const PREVIEW_STYLES: Record<string, string> = {
  paragraph: SAMPLE_TEXT,
  alphabet: ALPHABET,
  numbers: '0123456789 $€£¥',
  pangram: 'The quick brown fox jumps over the lazy dog'
};

const formatFileSize = (bytes: string): string => {
  const num = parseInt(bytes);
  if (isNaN(num)) return bytes;
  if (num < 1024) return `${num} B`;
  if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} KB`;
  return `${(num / (1024 * 1024)).toFixed(1)} MB`;
};

type TabType = 'preview' | 'metrics' | 'info' | 'pairing' | 'similar';

export const DetailPanel: React.FC<DetailPanelProps> = ({ font, onClose, library, onSelectFont, onPreview, hasPreviewPanel }) => {
  // Debug: Log font properties
  console.log('DetailPanel font:', {
    name: font?.name,
    primary_class: (font as any)?.primary_class,
    serif_subtype: (font as any)?.serif_subtype,
    sans_subtype: (font as any)?.sans_subtype,
    script_subtype: (font as any)?.script_subtype
  });

  const [activeTab, setActiveTab] = useState<TabType>('info');
  const [selectedSize, setSelectedSize] = useState('48px');
  const [selectedStyle, setSelectedStyle] = useState('paragraph');
  const [customText, setCustomText] = useState('Type here to preview...');
  const [customSize, setCustomSize] = useState(48);
  const [isRescanning, setIsRescanning] = useState(false);

  const { fontFamily, isLoading: fontLoading } = useFontLoader(font?.id || '', font?.filePath);

  if (!font) return null;

  const tabs: { key: TabType; label: string; icon: React.ReactNode }[] = [
    { key: 'info', label: 'Info', icon: <Info size={14} /> },
    { key: 'preview', label: 'Preview', icon: <Type size={14} /> },
    { key: 'metrics', label: 'Metrics', icon: <Layers size={14} /> },
    { key: 'pairing', label: 'Pairing', icon: <Users size={14} /> },
    { key: 'similar', label: 'Similar', icon: <Sparkles size={14} /> },
  ];

  return (
    <AnimatePresence>
      {font && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/20 backdrop-blur-[2px] z-40"
            onClick={onClose}
          />

          {/* Panel */}
          <motion.div
            initial={{ x: '100%' }}
            animate={{ x: 0 }}
            exit={{ x: '100%' }}
            transition={{ duration: 0.2, ease: 'easeOut' }}
            className={`fixed top-0 h-full bg-[var(--card)] shadow-[0_0_50px_rgba(0,0,0,0.1)] z-50 flex flex-col border-l border-[var(--border)] ${hasPreviewPanel ? 'right-[480px] w-full md:w-[580px]' : 'right-0 w-full md:w-[580px]'
              }`}
          >
            {/* Header */}
            <div className="bg-stone-50 text-stone-900 p-6 shrink-0 shadow-sm border-b border-stone-200">
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-2 flex-wrap">
                  {(font as any).primary_class && (
                    <span className="px-2 py-0.5 bg-stone-900/5 text-stone-600 text-[11px] rounded-full border border-stone-200 font-medium uppercase tracking-wider">
                      {(font as any).primary_class}
                    </span>
                  )}
                  {(font as any).serif_subtype && (
                    <span className="px-2 py-0.5 bg-amber-50 text-amber-800 text-[11px] rounded-full border border-amber-200 font-medium uppercase tracking-wider">
                      {(font as any).serif_subtype}
                    </span>
                  )}
                  {(font as any).sans_subtype && (
                    <span className="px-2 py-0.5 bg-blue-50 text-blue-800 text-[11px] rounded-full border border-blue-200 font-medium uppercase tracking-wider">
                      {(font as any).sans_subtype}
                    </span>
                  )}
                  {(font as any).script_subtype && (
                    <span className="px-2 py-0.5 bg-purple-50 text-purple-800 text-[11px] rounded-full border border-purple-200 font-medium uppercase tracking-wider">
                      {(font as any).script_subtype}
                    </span>
                  )}
                  <span className="text-stone-400 text-[11px] font-medium">{formatFileSize(font.fileSize)}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="icon"
                    onClick={async () => {
                      if (!font.id) return;
                      try {
                        const newStatus = !font.isFavorite;
                        await toggleFavorite(font.id, newStatus);
                        (font as any).isFavorite = newStatus;
                      } catch (e) {
                        console.error('Failed to toggle favorite:', e);
                      }
                    }}
                    className={`!p-1.5 transition-colors ${font.isFavorite ? 'text-rose-500 hover:text-rose-600' : 'text-stone-400 hover:text-stone-600'}`}
                  >
                    <Heart size={18} className={font.isFavorite ? "fill-current" : ""} />
                  </Button>
                  <Button
                    variant="icon"
                    onClick={async () => {
                      if (!font || isRescanning || !font.id) return;
                      setIsRescanning(true);
                      try { await rescanFont(font.id); }
                      catch (e) { console.error('Rescan failed:', e); }
                      finally { setIsRescanning(false); }
                    }}
                    className="!text-stone-400 hover:!text-stone-600 !p-1.5 transition-colors"
                  >
                    {isRescanning ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />}
                  </Button>
                  <Button variant="icon" onClick={onClose} className="!text-stone-400 hover:!text-stone-600 !p-1.5 transition-colors">
                    <X size={18} />
                  </Button>
                </div>
              </div>

              <h2 className="text-3xl font-semibold tracking-tight leading-tight text-stone-900">{font.name}</h2>
              <p className="text-stone-500 text-xs mt-1 font-mono-code opacity-80 uppercase tracking-widest">{font.family} • {font.category}</p>

              {/* Quick Stats Row */}
              <div className="grid grid-cols-4 gap-2 mt-4 text-center">
                <QuickStat label="x-height" value={font.metrics.xHeightRatio.toFixed(2)} />
                <QuickStat label="contrast" value={font.metrics.strokeContrast.toFixed(1)} />
                <QuickStat label="width" value={font.metrics.widthVariance.toFixed(2)} />
                <QuickStat label="aperture" value={font.metrics.aperture.toFixed(2)} />
              </div>
            </div>

            {/* Tabs Navigation */}
            <div className="flex border-b border-[var(--border)] px-4 bg-[var(--surface-2)] shrink-0 overflow-x-auto no-scrollbar">
              {tabs.map((tab) => (
                <button
                  key={tab.key}
                  onClick={() => setActiveTab(tab.key)}
                  className={`flex items-center gap-2 py-3 px-4 text-[11px] font-medium border-b-2 transition-all whitespace-nowrap ${activeTab === tab.key
                    ? 'border-[var(--foreground)] text-[var(--foreground)]'
                    : 'border-transparent text-[var(--muted)] hover:text-[var(--foreground)]'
                    }`}
                >
                  {tab.icon}
                  {tab.label}
                </button>
              ))}
            </div>

            {/* Main Content Area */}
            <div className="flex-1 overflow-y-auto custom-scroll">
              <AnimatePresence mode="wait">
                <motion.div
                  key={activeTab}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.2 }}
                  className="p-6"
                >
                  {activeTab === 'info' && <FontInfoTab font={font} fontFamily={fontFamily} fontLoading={fontLoading} />}
                  {activeTab === 'preview' && <PreviewSection font={font} fontFamily={fontFamily} fontLoading={fontLoading} selectedSize={selectedSize} setSelectedSize={setSelectedSize} selectedStyle={selectedStyle} setSelectedStyle={setSelectedStyle} customText={customText} setCustomText={setCustomText} customSize={customSize} setCustomSize={setCustomSize} />}
                  {activeTab === 'metrics' && <MetricsSection font={font} />}
                  {activeTab === 'pairing' && <PairingTab font={font} fontFamily={fontFamily} onSelectFont={onSelectFont} onPreview={onPreview} library={library} />}
                  {activeTab === 'similar' && <SimilarTab font={font} library={library} onSelectFont={onSelectFont} onPreview={onPreview} />}
                </motion.div>
              </AnimatePresence>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
};

// ============================================================================
// SUB-COMPONENTS
// ============================================================================

const QuickStat: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="bg-white rounded-lg py-3 px-1 border border-stone-200">
    <span className="text-xl font-semibold tracking-tighter text-stone-900">{value}</span>
    <p className="text-[10px] text-stone-400 uppercase tracking-widest mt-0.5 font-bold">{label}</p>
  </div>
);

const PreviewSection: React.FC<any> = ({ font, fontFamily, fontLoading, selectedSize, setSelectedSize, selectedStyle, setSelectedStyle, customText, setCustomText, customSize, setCustomSize }) => (
  <div className="space-y-6">
    <div className="space-y-3">
      <div className="flex items-center gap-2 flex-wrap">
        {PREVIEW_SIZES.map((s) => (
          <button key={s.label} onClick={() => setSelectedSize(s.size)} className={`px-2.5 py-1 text-[10px] rounded border transition-all ${selectedSize === s.size ? 'bg-[var(--foreground)] text-[var(--background)] border-[var(--foreground)]' : 'bg-[var(--surface-1)] text-[var(--muted)] border-[var(--border)] hover:border-[var(--muted)]'}`}>{s.label}</button>
        ))}
        <div className="w-px h-4 bg-[var(--border)] mx-1" />
        {Object.keys(PREVIEW_STYLES).map((style) => (
          <button key={style} onClick={() => setSelectedStyle(style)} className={`px-2.5 py-1 text-[10px] rounded border capitalize transition-all ${selectedStyle === style ? 'bg-[var(--foreground)] text-[var(--background)] border-[var(--foreground)]' : 'bg-[var(--surface-1)] text-[var(--muted)] border-[var(--border)] hover:border-[var(--muted)]'}`}>{style}</button>
        ))}
      </div>

      {fontLoading ? (
        <div className="p-10 bg-[var(--surface-2)] text-[var(--muted)] text-center animate-pulse rounded border border-[var(--border)]">Loading typeface...</div>
      ) : (
        <div className="p-10 bg-[var(--surface-2)] text-[var(--foreground)] leading-tight min-h-[140px] break-words rounded border border-[var(--border)] shadow-sm" style={{ fontFamily, fontSize: selectedSize }}>
          {PREVIEW_STYLES[selectedStyle]}
        </div>
      )}
    </div>

    <div className="pt-6 border-t border-[var(--border)] space-y-4">
      <h4 className="text-[10px] font-semibold text-[var(--muted)] uppercase tracking-widest">Interactive Playground</h4>
      <textarea value={customText} onChange={(e) => setCustomText(e.target.value)} className="w-full p-3 bg-[var(--surface-1)] border border-[var(--border)] rounded-lg text-sm focus:outline-none focus:ring-1 focus:ring-[var(--muted)] transition-all" rows={2} placeholder="Test your text here..." />
      <div className="flex items-center gap-4">
        <span className="text-[10px] font-mono text-[var(--muted)] w-12">{customSize}px</span>
        <input type="range" min="12" max="180" value={customSize} onChange={(e) => setCustomSize(Number(e.target.value))} className="flex-1 h-1 bg-[var(--border)] rounded-lg accent-[var(--foreground)] cursor-pointer" />
      </div>
      <div className="p-8 bg-[var(--surface-3)] border border-[var(--border)] rounded-lg min-h-[80px]" style={{ fontFamily, fontSize: `${customSize}px`, lineHeight: 1.1 }}>
        {customText}
      </div>
    </div>
  </div>
);

const MetricsSection: React.FC<{ font: FontData }> = ({ font }) => (
  <div className="space-y-6">
    <div className="grid grid-cols-2 gap-3">
      <MetricCard label="X-Height Ratio" value={font.metrics.xHeightRatio.toFixed(3)} desc="height_x / height_cap" range="[0.45–0.75]" hint={font.metrics.xHeightRatio > 0.58 ? '↑ tall' : font.metrics.xHeightRatio < 0.45 ? '↓ short' : '◦ std'} />
      <MetricCard label="Stroke Contrast" value={font.metrics.strokeContrast.toFixed(2)} desc="max / min stroke" range="[1.0–8.0+]" hint={font.metrics.strokeContrast < 1.3 ? '◦ mono' : font.metrics.strokeContrast > 3 ? '⬆ high' : '↗ low'} />
      <MetricCard label="Width Variance" value={font.metrics.widthVariance.toFixed(3)} desc="σ(glyph_widths)" range="[0.0–0.5+]" hint={font.metrics.widthVariance < 0.05 ? '▬ mono' : '↔ prop'} />
      <MetricCard label="Aperture" value={font.metrics.aperture.toFixed(2)} desc="c, e, s opening" range="[0.0–1.0]" hint={font.metrics.aperture > 0.6 ? '○ open' : font.metrics.aperture < 0.4 ? '◉ closed' : '◦ mod'} />
    </div>

    <div className="grid grid-cols-4 gap-2">
      <CompactMetric label="Curvature" value={font.metrics.curvature.toFixed(2)} />
      <CompactMetric label="Terminal" value={font.metrics.terminalSharpness.toFixed(2)} />
      <CompactMetric label="Stress" value={`${font.metrics.stressAngle.toFixed(0)}°`} />
      <CompactMetric label="Counter" value={font.metrics.counterDensity.toFixed(2)} />
    </div>

    <div className="p-4 bg-stone-50 border border-stone-200 rounded-xl">
      <MetricsChart metrics={font.metrics} />
    </div>
  </div>
);

const MetricCard: React.FC<any> = ({ label, value, desc, range, hint }) => (
  <div className="bg-white rounded-xl p-4 border border-stone-200 shadow-sm hover:border-stone-400 transition-colors">
    <div className="flex justify-between items-start mb-1">
      <span className="text-[10px] font-bold text-stone-400 uppercase tracking-tighter">{label}</span>
      <span className="text-xl font-semibold text-stone-900">{value}</span>
    </div>
    <p className="text-[9px] font-mono-code text-stone-400 mb-2 opacity-70">{desc}</p>
    <div className="flex items-center gap-2">
      <code className="text-[8px] text-stone-500 bg-stone-100 px-1.5 py-0.5 rounded">{range}</code>
      <span className="text-[9px] font-medium text-stone-600 opacity-60 italic">{hint}</span>
    </div>
  </div>
);

const CompactMetric: React.FC<any> = ({ label, value }) => (
  <div className="bg-stone-50 rounded-lg p-3 text-center border border-stone-200">
    <span className="text-xs font-semibold text-stone-800">{value}</span>
    <p className="text-[7px] text-stone-400 uppercase tracking-widest mt-1 font-bold">{label}</p>
  </div>
);

const FontInfoTab: React.FC<{ font: FontData; fontFamily?: string; fontLoading?: boolean }> = ({ font, fontFamily, fontLoading }) => (
  <div className="space-y-8">
    <div className="bg-stone-50/50 rounded-2xl border border-stone-200/60 p-6 space-y-8">
      <section>
        <h3 className="text-[10px] font-bold text-stone-400 uppercase tracking-[0.2em] mb-4">Metadata</h3>
        <dl className="grid grid-cols-2 gap-x-8 gap-y-6 text-sm">
          <InfoItem label="Full Name" value={font.name} />
          <InfoItem label="Family" value={font.family} />
          <InfoItem label="Category" value={font.category} />
          <InfoItem label="Foundry" value={font.foundry || 'Unknown'} />
          <InfoItem label="File Size" value={formatFileSize(font.fileSize)} />
          <InfoItem label="Analyzed" value={font.dateAnalyzed} mono />
        </dl>
      </section>

      {(font as any).primary_class && (
        <section className="pt-6 border-t border-stone-200/60">
          <h3 className="text-[10px] font-bold text-stone-400 uppercase tracking-[0.2em] mb-4">Classification</h3>
          <div className="flex flex-wrap gap-2">
            <Badge label={(font as any).primary_class} accent />
            {(font as any).serif_subtype && <Badge label={(font as any).serif_subtype} />}
            {(font as any).sans_subtype && <Badge label={(font as any).sans_subtype} />}
            {(font as any).script_subtype && <Badge label={(font as any).script_subtype} />}
          </div>
        </section>
      )}

      {font.secondary_tags && (Array.isArray(font.secondary_tags) ? font.secondary_tags.length > 0 : font.secondary_tags) && (
        <section className="pt-6 border-t border-stone-200/60">
          <h3 className="text-[10px] font-bold text-stone-400 uppercase tracking-[0.2em] mb-4">Functional Tags</h3>
          <div className="flex flex-wrap gap-2">
            {(Array.isArray(font.secondary_tags) ? font.secondary_tags : JSON.parse(font.secondary_tags || '[]')).map((tag: string, idx: number) => (
              <Badge key={idx} label={tag} />
            ))}
          </div>
        </section>
      )}
    </div>

    {/* Mini Specimen */}
    <section className="pt-6 border-t border-stone-200">
      <div className="bg-stone-50 rounded-2xl p-6 border border-stone-200 overflow-hidden relative">
        <div className="absolute top-3 right-4 text-[10px] uppercase tracking-widest font-bold text-stone-400 opacity-50">Quick Specimen</div>
        {fontLoading ? (
          <div className="h-24 animate-pulse bg-stone-100 rounded-lg flex items-center justify-center">
            <Loader2 className="animate-spin text-stone-300" />
          </div>
        ) : (
          <div className="space-y-4">
            <div style={{ fontFamily }} className="text-5xl leading-none text-stone-900 truncate">
              {font.name}
            </div>
            <div style={{ fontFamily }} className="text-xl leading-relaxed text-stone-600 line-clamp-2">
              The quick brown fox jumps over the lazy dog. 0123456789
            </div>
          </div>
        )}
      </div>
    </section>

    <section className="pt-6 border-t border-stone-200">
      <h3 className="text-[10px] font-bold text-stone-400 uppercase tracking-[0.2em] mb-4">External Resources</h3>
      <div className="grid grid-cols-1 gap-2">
        <ExternalLinkItem label="Search on Google Fonts" href={`https://fonts.google.com/specimen/${encodeURIComponent(font.family.replace(/ /g, '+'))}`} />
        <ExternalLinkItem label="Search on MyFonts" href={`https://www.myfonts.com/search/${encodeURIComponent(font.family)}`} />
      </div>
    </section>
  </div>
);

const InfoItem: React.FC<any> = ({ label, value, mono }) => (
  <div>
    <dt className="text-[9px] text-[var(--muted)] uppercase tracking-widest mb-1">{label}</dt>
    <dd className={`text-[var(--foreground)] font-medium ${mono ? 'font-mono-code text-xs' : ''}`}>{value}</dd>
  </div>
);

const Badge: React.FC<any> = ({ label, accent }) => (
  <span className={`px-3 py-1 text-[10px] rounded-full border transition-all ${accent ? 'bg-amber-50 text-amber-700 border-amber-200' : 'bg-[var(--surface-2)] text-[var(--muted)] border-[var(--border)]'}`}>
    {label}
  </span>
);

const ExternalLinkItem: React.FC<any> = ({ label, href }) => (
  <a href={href} target="_blank" rel="noopener noreferrer" className="flex items-center justify-between p-3 bg-[var(--surface-2)] border border-[var(--border)] rounded-lg text-xs text-[var(--muted)] hover:text-[var(--foreground)] hover:border-[var(--muted)] hover:bg-[var(--surface-1)] transition-all group">
    <div className="flex items-center gap-3">
      <Globe size={14} className="opacity-50 group-hover:opacity-100" />
      {label}
    </div>
    <ExternalLink size={12} className="opacity-30 group-hover:opacity-100" />
  </a>
);

// ============================================================================
// Similar Tab Component
// ============================================================================

const SimilarTab: React.FC<{ font: FontData; library: FontData[]; onSelectFont?: (font: FontData) => void; onPreview?: (font: FontData) => void }> = ({ font, library, onSelectFont, onPreview }) => {
  const [similarFonts, setSimilarFonts] = useState<SimilarFont[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<'perceptual' | 'metrics'>('perceptual');

  useEffect(() => {
    const fetchSimilar = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const results = await qdrantService.findSimilarFonts(font.id, 12, mode);
        setSimilarFonts(results);
      } catch (err: any) {
        setError(err?.message || 'Failed to find similar fonts');
      } finally {
        setIsLoading(false);
      }
    };
    fetchSimilar();
  }, [font.id, mode]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-[10px] font-bold text-[var(--muted)] uppercase tracking-[0.2em]">Visual Relations</h3>
        <div className="flex bg-[var(--surface-2)] p-0.5 rounded-lg border border-[var(--border)]">
          <button
            onClick={() => setMode('perceptual')}
            className={`px-2 py-0.5 text-[9px] rounded-md transition-all font-bold uppercase tracking-widest ${mode === 'perceptual' ? 'bg-[var(--foreground)] text-[var(--background)] shadow-sm' : 'text-[var(--muted)] hover:text-[var(--foreground)]'}`}
          >
            Visual
          </button>
          <button
            onClick={() => setMode('metrics')}
            className={`px-2 py-0.5 text-[9px] rounded-md transition-all font-bold uppercase tracking-widest ${mode === 'metrics' ? 'bg-[var(--foreground)] text-[var(--background)] shadow-sm' : 'text-[var(--muted)] hover:text-[var(--foreground)]'}`}
          >
            Metrics
          </button>
        </div>
      </div>

      {isLoading && (
        <div className="flex flex-col items-center justify-center py-20 space-y-4">
          <Loader2 size={32} className="animate-spin text-[var(--muted)] opacity-20" />
          <p className="text-[10px] text-[var(--muted)] uppercase tracking-widest">Embedding Search...</p>
        </div>
      )}

      {error && <div className="bg-red-50 border border-red-100 rounded-lg p-5 text-xs text-red-600 leading-relaxed">{error}</div>}

      {!isLoading && !error && similarFonts.length === 0 && (
        <div className="text-center py-20 text-[var(--muted)] text-sm italic">
          {mode === 'perceptual'
            ? "No visual siblings found. Try switching to metrics."
            : "No typographic siblings found in library."}
        </div>
      )}

      <div className="grid grid-cols-1 gap-3">
        {similarFonts.map((item) => {
          const libFont = library.find(f => f.id === item.font_id);

          return (
            <motion.div
              key={item.font_id}
              layout
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              onClick={() => {
                console.log('Similar font clicked:', {
                  font_id: item.font_id,
                  library_size: library.length,
                  libFont_name: libFont?.name,
                  libFont_exists: !!libFont,
                  onPreview_exists: !!onPreview
                });
                if (libFont && onPreview) {
                  onPreview(libFont);
                } else if (!libFont) {
                  console.error('Font not found in library:', item.font_id);
                }
              }}
              className="flex items-center gap-5 p-4 rounded-xl border bg-[var(--surface-1)] border-[var(--border)] hover:border-[var(--muted)] hover:shadow-sm transition-all cursor-pointer group"
            >
              <SimilarFontPreview font_id={item.font_id} file_path={item.file_path} />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-[var(--foreground)] truncate">{item.name}</p>
                <p className="text-[10px] text-[var(--muted)] uppercase tracking-tighter">{item.category || 'Family'}</p>
              </div>
              <div className="text-right">
                <div className="text-sm font-mono-code text-[var(--foreground)]">{(item.similarity_score * 100).toFixed(1)}%</div>
                <div className="text-[8px] text-[var(--muted)] uppercase tracking-widest opacity-50">Match Score</div>
              </div>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
};

const SimilarFontPreview: React.FC<{ font_id: string; file_path: string }> = ({ font_id, file_path }) => {
  const { fontFamily, isLoading } = useFontLoader(font_id, file_path, true);
  return (
    <div className="w-14 h-10 flex items-center justify-center bg-[var(--surface-2)] rounded border border-[var(--border)]">
      {isLoading ? (
        <div className="w-4 h-4 rounded-full border border-[var(--muted)] border-t-transparent animate-spin opacity-20" />
      ) : (
        <span className="text-2xl text-[var(--foreground)] leading-none" style={{ fontFamily }}>Aa</span>
      )}
    </div>
  );
};