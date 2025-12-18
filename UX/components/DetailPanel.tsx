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

export const DetailPanel: React.FC<DetailPanelProps> = ({ font, onClose, library, children }) => {
  const [activeTab, setActiveTab] = useState<'anatomy' | 'pairing' | 'preview'>('anatomy');
  const [currentSampleIndex, setCurrentSampleIndex] = useState(0);

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
        <div className="flex-1 overflow-y-auto no-scrollbar p-8">

          {activeTab === 'preview' && children && (
            <div className="animate-in fade-in slide-in-from-bottom-2 duration-500">
              {children}
            </div>
          )}

          {activeTab === 'anatomy' && (
            <div className="space-y-12 animate-in fade-in slide-in-from-bottom-2 duration-500">
              {/* Type Specimen */}
              <section>
                <h3 className="text-xs uppercase tracking-widest text-stone-400 mb-6 flex items-center gap-2">
                  <Type size={14} /> Specimen
                </h3>
                {fontLoading ? (
                  <div className="p-8 border border-stone-100 bg-stone-50 text-stone-400 text-center animate-pulse">
                    Loading font preview...
                  </div>
                ) : (
                  <>
                    <div
                      className="p-8 border border-stone-100 bg-stone-50 text-stone-900 text-4xl leading-tight mb-4 break-words"
                      style={{ fontFamily }}
                    >
                      {ALPHABET}
                    </div>
                    <div
                      key={currentSampleIndex}
                      className="p-8 border border-stone-100 bg-white text-stone-800 text-lg leading-relaxed animate-text-slide flex items-center min-h-[180px]"
                      style={{ fontFamily }}
                    >
                      {SLIDESHOW_TEXTS[currentSampleIndex]}
                    </div>
                  </>
                )}
              </section>

              {/* Metrics */}
              <section>
                <h3 className="text-xs uppercase tracking-widest text-stone-400 mb-6 flex items-center gap-2">
                  <Layers size={14} /> Metric Analysis
                </h3>
                <div className="grid grid-cols-2 gap-6 mb-8">
                  <div className="bg-stone-50 p-4 border border-stone-100">
                    <span className="text-xs text-stone-500 block mb-1">X-Height Ratio</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.xHeightRatio.toFixed(3)}</span>
                  </div>
                  <div className="bg-stone-50 p-4 border border-stone-100">
                    <span className="text-xs text-stone-500 block mb-1">Stroke Contrast</span>
                    <span className="text-2xl font-light text-stone-900">{font.metrics.strokeContrast.toFixed(2)}</span>
                  </div>
                </div>

                <div className="bg-white border border-stone-100 p-6">
                  <h4 className="text-sm font-medium text-stone-900 mb-2">Anatomical Balance</h4>
                  <p className="text-sm text-stone-500 mb-4 font-light leading-relaxed">
                    Visualizing the structural relationship between x-height, contrast, width variance, and aperture openness.
                  </p>
                  <MetricsChart metrics={font.metrics} />
                </div>
              </section>

              {/* Info */}
              <section className="bg-stone-900 text-stone-300 p-6">
                <h3 className="text-xs uppercase tracking-widest text-stone-500 mb-4 flex items-center gap-2">
                  <Info size={14} /> Metadata
                </h3>
                <div className="grid grid-cols-2 gap-y-4 gap-x-8 text-sm font-mono-code">
                  <div>
                    <span className="block text-stone-500 text-xs mb-0.5">Family</span>
                    {font.family}
                  </div>
                  <div>
                    <span className="block text-stone-500 text-xs mb-0.5">Foundry</span>
                    Google Fonts
                  </div>
                  <div>
                    <span className="block text-stone-500 text-xs mb-0.5">Date Analyzed</span>
                    {font.dateAnalyzed}
                  </div>
                  <div>
                    <span className="block text-stone-500 text-xs mb-0.5">Size</span>
                    {font.fileSize}
                  </div>
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