import React, { useState, useMemo, useEffect } from 'react';
import { X, ExternalLink, Type, Layers, Heart, Info, ArrowRight } from 'lucide-react';
import { FontData, FontPairing } from '../types';
import { Button } from './Button';
import { MetricsChart } from './MetricsChart';
import { generatePairings } from '../utils/pairing';
import { PairedFontCard } from './PairedFontCard';
import { ALPHABET, SAMPLE_TEXT } from '../constants';
import { useFontLoader } from '../hooks/useFontLoader';

interface DetailPanelProps {
  font: FontData | null;
  onClose: () => void;
  library: FontData[];
  children?: React.ReactNode;
}

const SLIDESHOW_TEXTS = [
  "The quick brown fox jumps over the lazy dog. Typography is the craft of endowing human language with a durable visual form.",
  "A typeface is a collection of letters. A font is the physical embodiment of that collection. Good design is as little design as possible.",
  "Sphinx of black quartz, judge my vow. Design is not just what it looks like and feels like. Design is how it works.",
  "To read is to fly: it is to soar to a point of vantage which gives a view over wide terrains of history, human variety, ideas, shared experience."
];

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

// Helper to format file size
const formatFileSize = (bytes: string): string => {
  const num = parseInt(bytes);
  if (isNaN(num)) return bytes;
  if (num < 1024) return `${num} B`;
  if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} KB`;
  return `${(num / (1024 * 1024)).toFixed(1)} MB`;
};


export const DetailPanel: React.FC<DetailPanelProps> = ({ font, onClose, library, children }) => {
  const [activeTab, setActiveTab] = useState<'anatomy' | 'pairing' | 'preview'>('anatomy');
  const [currentSampleIndex, setCurrentSampleIndex] = useState(0);

  // Enhanced preview controls
  const [selectedSize, setSelectedSize] = useState('48px');
  const [selectedStyle, setSelectedStyle] = useState('paragraph');
  const [customText, setCustomText] = useState('Type here to preview this font...');
  const [customSize, setCustomSize] = useState(48);

  // Load font for preview
  const { fontFamily, isLoading: fontLoading } = useFontLoader(font?.id || '', font?.filePath);

  // Find pairings for this font
  const pairings = useMemo(() => {
    if (!font || !library || library.length === 0) return [];
    return generatePairings(font, library);
  }, [font, library]);

  // Slideshow interval
  useEffect(() => {
    if (!font) return;
    const interval = setInterval(() => {
      setCurrentSampleIndex((prev) => (prev + 1) % SLIDESHOW_TEXTS.length);
    }, 6000); // Matches the 6s CSS animation duration (animate-text-slide)
    return () => clearInterval(interval);
  }, [font]);

  if (!font) return null;

  if (!font) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-stone-900/10 backdrop-blur-[1px] z-40 transition-opacity duration-300"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="fixed top-0 right-0 h-full w-full md:w-[600px] bg-white shadow-2xl z-50 transform transition-transform duration-400 cubic-bezier(0.4, 0, 0.2, 1) flex flex-col border-l border-stone-200">

        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-stone-100 bg-white/95 backdrop-blur z-10">
          <div>
            <span className="text-xs uppercase tracking-widest text-stone-500 font-medium mb-1 block">
              {font.category}
            </span>
            <h2 className="text-3xl font-light text-stone-900 tracking-tight">{font.name}</h2>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="icon" onClick={() => console.log('Favorite')}>
              <Heart size={18} />
            </Button>
            <Button variant="icon" onClick={onClose}>
              <X size={20} />
            </Button>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-stone-100 px-6">
          <button
            onClick={() => setActiveTab('anatomy')}
            className={`py-4 px-4 text-sm font-medium transition-colors border-b-2 ${activeTab === 'anatomy'
              ? 'border-stone-900 text-stone-900'
              : 'border-transparent text-stone-400 hover:text-stone-600'
              }`}
          >
            Anatomy & Metrics
          </button>
          {children && (
            <button
              onClick={() => setActiveTab('preview')}
              className={`py-4 px-4 text-sm font-medium transition-colors border-b-2 ${activeTab === 'preview'
                ? 'border-stone-900 text-stone-900'
                : 'border-transparent text-stone-400 hover:text-stone-600'
                }`}
            >
              Preview
            </button>
          )}
          <button
            onClick={() => setActiveTab('pairing')}
            className={`py-4 px-4 text-sm font-medium transition-colors border-b-2 ${activeTab === 'pairing'
              ? 'border-stone-900 text-stone-900'
              : 'border-transparent text-stone-400 hover:text-stone-600'
              }`}
          >
            AI Pairing <span className="ml-1 text-[10px] bg-stone-100 px-1.5 py-0.5 rounded-full text-stone-500">BETA</span>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto detail-scroll p-8">

          {activeTab === 'preview' && children && (
            <div className="animate-in fade-in slide-in-from-bottom-2 duration-500">
              {children}
            </div>
          )}

          {activeTab === 'anatomy' && (
            <div className="space-y-10 animate-in fade-in slide-in-from-bottom-2 duration-500">

              {/* Section 1: Enhanced Type Specimen with Controls */}
              <section>
                <h3 className="text-xs uppercase tracking-widest text-stone-400 mb-4 flex items-center gap-2">
                  <Type size={14} /> Type Specimen
                </h3>

                {/* Size Selector */}
                <div className="flex gap-2 mb-4 flex-wrap">
                  {PREVIEW_SIZES.map((size) => (
                    <button
                      key={size.label}
                      onClick={() => setSelectedSize(size.size)}
                      className={`px-3 py-1.5 text-xs rounded border transition-colors ${selectedSize === size.size
                        ? 'bg-stone-900 text-white border-stone-900'
                        : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'
                        }`}
                    >
                      {size.label}
                    </button>
                  ))}
                </div>

                {/* Style Selector */}
                <div className="flex gap-2 mb-6 flex-wrap">
                  {Object.keys(PREVIEW_STYLES).map((style) => (
                    <button
                      key={style}
                      onClick={() => setSelectedStyle(style)}
                      className={`px-3 py-1.5 text-xs rounded border capitalize transition-colors ${selectedStyle === style
                        ? 'bg-stone-900 text-white border-stone-900'
                        : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'
                        }`}
                    >
                      {style}
                    </button>
                  ))}
                </div>

                {/* Preview Area */}
                {fontLoading ? (
                  <div className="p-8 border border-stone-100 bg-stone-50 text-stone-400 text-center animate-pulse">
                    Loading font preview...
                  </div>
                ) : (
                  <div
                    className="p-8 border border-stone-100 bg-stone-50 text-stone-900 leading-tight min-h-[120px] break-words"
                    style={{ fontFamily, fontSize: selectedSize }}
                  >
                    {PREVIEW_STYLES[selectedStyle]}
                  </div>
                )}
              </section>

              {/* Section 2: Custom Preview Input */}
              <section className="bg-white border border-stone-200 rounded-lg p-6">
                <h3 className="text-xs uppercase tracking-widest text-stone-400 mb-4 flex items-center gap-2">
                  <Type size={14} /> Custom Preview
                </h3>

                <div className="space-y-4">
                  {/* Text Input */}
                  <textarea
                    value={customText}
                    onChange={(e) => setCustomText(e.target.value)}
                    className="w-full p-3 border border-stone-300 rounded text-sm font-mono focus:outline-none focus:border-stone-500 transition-colors"
                    rows={3}
                    placeholder="Type your text here..."
                  />

                  {/* Size Slider */}
                  <div className="flex items-center gap-4">
                    <label className="text-xs text-stone-500 font-medium min-w-[80px]">
                      Size: {customSize}px
                    </label>
                    <input
                      type="range"
                      min="12"
                      max="120"
                      value={customSize}
                      onChange={(e) => setCustomSize(Number(e.target.value))}
                      className="flex-1 h-2 bg-stone-200 rounded-lg appearance-none cursor-pointer accent-stone-900"
                    />
                  </div>

                  {/* Live Preview */}
                  <div
                    className="p-6 bg-stone-50 border border-stone-300 rounded min-h-[120px] overflow-auto"
                    style={{
                      fontFamily,
                      fontSize: `${customSize}px`,
                      lineHeight: 1.4
                    }}
                  >
                    {customText}
                  </div>
                </div>
              </section>

              {/* Section 3: Technical Specifications Table */}
              <section className="bg-stone-50 border border-stone-200 rounded-lg p-6">
                <h3 className="text-xs uppercase tracking-widest text-stone-400 mb-6 flex items-center gap-2">
                  <Layers size={14} /> Technical Specifications
                </h3>

                <div className="grid grid-cols-2 gap-x-8 gap-y-4 text-sm">
                  <div className="bg-white p-4 rounded border border-stone-200">
                    <span className="text-xs text-stone-500 block mb-1">X-Height Ratio</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.xHeightRatio.toFixed(3)}</span>
                    <p className="text-xs text-stone-400 mt-1">Lowercase height proportion</p>
                  </div>

                  <div className="bg-white p-4 rounded border border-stone-200">
                    <span className="text-xs text-stone-500 block mb-1">Stroke Contrast</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.strokeContrast.toFixed(2)}</span>
                    <p className="text-xs text-stone-400 mt-1">Thick/thin stroke variation</p>
                  </div>

                  <div className="bg-white p-4 rounded border border-stone-200">
                    <span className="text-xs text-stone-500 block mb-1">Width Variance</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.widthVariance.toFixed(2)}</span>
                    <p className="text-xs text-stone-400 mt-1">Character width variation</p>
                  </div>

                  <div className="bg-white p-4 rounded border border-stone-200">
                    <span className="text-xs text-stone-500 block mb-1">Aperture</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.aperture.toFixed(2)}</span>
                    <p className="text-xs text-stone-400 mt-1">Opening size in letters</p>
                  </div>
                </div>

                {/* Metrics Chart */}
                <div className="mt-6 bg-white border border-stone-200 p-6 rounded">
                  <h4 className="text-sm font-medium text-stone-900 mb-2">Anatomical Balance</h4>
                  <p className="text-sm text-stone-500 mb-4 font-light leading-relaxed">
                    Visualizing the structural relationship between key typographic metrics.
                  </p>
                  <MetricsChart metrics={font.metrics} />
                </div>
              </section>

              {/* Section 4: Full Metadata */}
              <section className="bg-stone-900 text-stone-300 rounded-lg p-6">
                <h3 className="text-xs uppercase tracking-widest text-stone-500 mb-6 flex items-center gap-2">
                  <Info size={14} /> Font Information
                </h3>

                <div className="grid grid-cols-2 gap-x-8 gap-y-6">
                  {/* Classification (New Taxonomy) */}
                  {((font as any).primary_class || (font as any).secondary_tags) && (
                    <div className="col-span-2">
                      <h4 className="text-xs text-stone-500 uppercase tracking-wider mb-3">Classification</h4>
                      <div className="space-y-3">
                        {/* Primary Class & Subtype */}
                        {(font as any).primary_class && (
                          <div>
                            <dt className="text-stone-500 text-xs mb-1">Type</dt>
                            <dd className="flex items-center gap-2 flex-wrap">
                              <span className="px-3 py-1 bg-amber-500/20 text-amber-300 text-sm rounded-full border border-amber-500/30 font-medium capitalize">
                                {(font as any).primary_class}
                              </span>
                              {(font as any).serif_subtype && (
                                <span className="px-3 py-1 bg-stone-700 text-stone-300 text-xs rounded-full border border-stone-600 capitalize">
                                  {(font as any).serif_subtype}
                                </span>
                              )}
                              {(font as any).sans_subtype && (
                                <span className="px-3 py-1 bg-stone-700 text-stone-300 text-xs rounded-full border border-stone-600 capitalize">
                                  {(font as any).sans_subtype}
                                </span>
                              )}
                              {(font as any).script_subtype && (
                                <span className="px-3 py-1 bg-stone-700 text-stone-300 text-xs rounded-full border border-stone-600 capitalize">
                                  {(font as any).script_subtype}
                                </span>
                              )}
                            </dd>
                          </div>
                        )}

                        {/* Secondary Tags */}
                        {(font as any).secondary_tags && (
                          <div>
                            <dt className="text-stone-500 text-xs mb-1">Characteristics</dt>
                            <dd className="flex flex-wrap gap-2">
                              {(() => {
                                try {
                                  const tags = JSON.parse((font as any).secondary_tags);
                                  return tags.map((tag: string) => (
                                    <span
                                      key={tag}
                                      className="px-2 py-0.5 bg-stone-800 text-stone-400 text-xs rounded border border-stone-700 hover:border-stone-600 transition-colors"
                                    >
                                      {tag}
                                    </span>
                                  ));
                                } catch {
                                  return null;
                                }
                              })()}
                            </dd>
                          </div>
                        )}
                      </div>
                    </div>
                  )}

                  {/* Basic Info */}
                  <div>
                    <h4 className="text-xs text-stone-500 uppercase tracking-wider mb-3">Basic Info</h4>
                    <dl className="space-y-2 text-sm">
                      <div>
                        <dt className="text-stone-500 text-xs">Font Name</dt>
                        <dd className="text-stone-200 font-medium">{font.name}</dd>
                      </div>
                      <div>
                        <dt className="text-stone-500 text-xs">Family</dt>
                        <dd className="text-stone-200">{font.family}</dd>
                      </div>
                      <div>
                        <dt className="text-stone-500 text-xs">Category</dt>
                        <dd className="text-stone-200">{font.category}</dd>
                      </div>
                    </dl>
                  </div>

                  {/* Technical Details */}
                  <div>
                    <h4 className="text-xs text-stone-500 uppercase tracking-wider mb-3">Technical Details</h4>
                    <dl className="space-y-2 text-sm">
                      <div>
                        <dt className="text-stone-500 text-xs">Date Analyzed</dt>
                        <dd className="text-stone-200 font-mono text-xs">{font.dateAnalyzed}</dd>
                      </div>
                      <div>
                        <dt className="text-stone-500 text-xs">File Size</dt>
                        <dd className="text-stone-200">{formatFileSize(font.fileSize)}</dd>
                      </div>
                      <div>
                        <dt className="text-stone-500 text-xs">Foundry</dt>
                        <dd className="text-stone-200">{(font as any).foundry || 'Unknown'}</dd>
                      </div>
                    </dl>
                  </div>

                  {/* Tags */}
                  {font.tags && font.tags.length > 0 && (
                    <div className="col-span-2">
                      <h4 className="text-xs text-stone-500 uppercase tracking-wider mb-3">Tags</h4>
                      <div className="flex flex-wrap gap-2">
                        {font.tags.map((tag) => (
                          <span
                            key={tag}
                            className="px-2 py-1 bg-stone-800 text-stone-300 text-xs rounded border border-stone-700"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </section>
            </div>
          )}

          {activeTab === 'pairing' && (
            <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-500">
              <div className="bg-stone-50 p-6 border border-stone-100 flex items-start gap-4">
                <div className="bg-white p-2 border border-stone-200 shadow-sm">
                  <span className="text-2xl" style={{ fontFamily: font.family }}>Aa</span>
                </div>
                <div>
                  <h4 className="font-medium text-stone-900">Pairing Generator</h4>
                  <p className="text-sm text-stone-500 mt-1 leading-relaxed">
                    Based on {font.name}'s anatomy ({font.metrics.strokeContrast} contrast, {font.category}),
                    AI suggests pairings with high harmonic scores.
                  </p>
                </div>
              </div>

              {pairings.length > 0 ? (
                pairings.map((pair, idx) => (
                  <PairedFontCard
                    key={`${pair.targetFontId}-${idx}`}
                    primaryFont={font}
                    primaryFontFamily={fontFamily}
                    secondaryFont={pair.font!} // generatePairings attaches 'font' object
                    score={pair.score}
                    rationale={pair.rationale}
                  />
                ))
              ) : (
                <div className="text-center py-12 border border-dashed border-stone-300">
                  <p className="text-stone-500">No AI pairings generated for this font yet.</p>
                  <p className="text-xs text-stone-400 mt-2">Try scanning more fonts to build a library.</p>
                </div>
              )}
            </div>
          )}

        </div>
      </div>
    </>
  );
};