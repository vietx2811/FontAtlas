import React, { useState, useEffect } from 'react';
import { FontData } from '../types';
import { qdrantService, PairingByLogic, FontCombo } from '../services/qdrant';
import { Loader2, AlertCircle, ChevronDown, ChevronRight, Sparkles, Layers, Cpu, Eye } from 'lucide-react';
import { useFontLoader } from '../hooks/useFontLoader';
import { AnimatePresence } from 'framer-motion';
import { FontActionPanel } from './FontActionPanel';

interface PairingTabProps {
    font: FontData;
    fontFamily: string;
    onSelectFont?: (font: FontData) => void;
    onPreview?: (font: FontData) => void;
    library: FontData[];
}

// Icons for each pairing logic
const LOGIC_ICONS: Record<string, React.ReactNode> = {
    contrast: <Sparkles size={18} />,
    harmony: <Layers size={18} />,
    modern: <Cpu size={18} />,
};

export const PairingTab: React.FC<PairingTabProps> = ({ font, fontFamily, onSelectFont, onPreview, library }) => {
    const [logicGroups, setLogicGroups] = useState<PairingByLogic[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [expandedLogics, setExpandedLogics] = useState<Set<string>>(new Set(['contrast']));
    const [previewCombo, setPreviewCombo] = useState<FontCombo | null>(null);

    useEffect(() => {
        const fetchCombos = async () => {
            setIsLoading(true);
            setError(null);

            try {
                const results = await qdrantService.findFontCombos(font.id, 5);
                setLogicGroups(results);
                // Auto-expand first logic with results
                const firstWithResults = results.find(r => r.results.length > 0);
                if (firstWithResults) {
                    setExpandedLogics(new Set([firstWithResults.logic_key]));
                }
            } catch (err: any) {
                console.error('Failed to load combos:', err);
                setError(err?.message || 'Failed to load font pairings');
            } finally {
                setIsLoading(false);
            }
        };

        fetchCombos();
    }, [font.id]);

    const toggleLogic = (key: string) => {
        setExpandedLogics(prev => {
            const next = new Set(prev);
            if (next.has(key)) {
                next.delete(key);
            } else {
                next.add(key);
            }
            return next;
        });
    };

    return (
        <div className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-500">
            {/* Header */}
            <div className="bg-gradient-to-r from-stone-50 to-amber-50/30 p-5 border border-stone-200 rounded-lg flex items-start gap-4">
                <div className="bg-white p-2 border border-stone-200 shadow-sm rounded">
                    <span className="text-2xl" style={{ fontFamily: font.family }}>Aa</span>
                </div>
                <div>
                    <h4 className="font-semibold text-stone-900">AI Font Combinations</h4>
                    <p className="text-sm text-stone-500 mt-1 leading-relaxed">
                        Multiple pairing strategies for {font.name}. Click to preview with different roles.
                    </p>
                </div>
            </div>

            {/* Loading State */}
            {isLoading && (
                <div className="flex items-center justify-center py-16">
                    <Loader2 size={32} className="animate-spin text-amber-500" />
                    <span className="ml-3 text-stone-500">Analyzing pairings...</span>
                </div>
            )}

            {/* Error State */}
            {error && !isLoading && (
                <div className="bg-red-50 border border-red-200 rounded-lg p-6">
                    <div className="flex items-start gap-3">
                        <AlertCircle size={20} className="text-red-600 mt-0.5" />
                        <div>
                            <h5 className="font-medium text-red-900">Unable to load pairings</h5>
                            <p className="text-sm text-red-700 mt-1">{error}</p>
                        </div>
                    </div>
                </div>
            )}

            {/* Results - Editorial Single Column Layout */}
            {!isLoading && !error && logicGroups.length > 0 && (
                <div className="space-y-8">
                    {logicGroups.map((logic) => (
                        <div key={logic.logic_key} className="space-y-4">
                            {/* Logic Header - Editorial Style */}
                            <div className="border-b-2 border-stone-800 pb-2">
                                <h3 className="text-lg font-bold text-stone-900 tracking-tight">{logic.logic_name}</h3>
                                <p className="text-xs text-stone-500 mt-0.5">{logic.description}</p>
                            </div>

                            {/* Combos List - Editorial Format */}
                            {logic.results.length > 0 ? (
                                <div className="space-y-1">
                                    {logic.results.map((combo, idx) => (
                                        <div
                                            key={idx}
                                            className="group flex items-center gap-4 py-3 border-b border-stone-100 hover:bg-stone-50 transition-colors cursor-pointer"
                                            onClick={() => setPreviewCombo(combo)}
                                        >
                                            {/* Number */}
                                            <span className="text-[10px] font-mono text-stone-400 w-4">{idx + 1}.</span>

                                            {/* Font Names */}
                                            <div className="flex-1 space-y-0.5">
                                                {combo.fonts.map((font, fIdx) => {
                                                    const libFont = library.find(f => f.id === font.font_id);
                                                    return (
                                                        <div
                                                            key={fIdx}
                                                            className="flex items-center gap-3"
                                                            onClick={(e) => {
                                                                e.stopPropagation();
                                                                if (libFont && onPreview) onPreview(libFont);
                                                            }}
                                                        >
                                                            <span className="text-[9px] uppercase tracking-wider text-stone-400 w-16">{font.role}</span>
                                                            <span className="text-sm font-medium text-stone-800 hover:text-amber-700 transition-colors">{font.font_name}</span>
                                                        </div>
                                                    );
                                                })}
                                            </div>

                                            {/* Preview Demo */}
                                            <div className="hidden md:block flex-1 text-right pr-4">
                                                <span
                                                    className="text-sm text-stone-400"
                                                    style={{ fontFamily: combo.fonts[0]?.font_name || 'inherit' }}
                                                >
                                                    Heading
                                                </span>
                                                {combo.fonts.length > 1 && (
                                                    <span
                                                        className="text-xs text-stone-300 ml-2"
                                                        style={{ fontFamily: combo.fonts[1]?.font_name || 'inherit' }}
                                                    >
                                                        Body text
                                                    </span>
                                                )}
                                            </div>

                                            {/* View Button */}
                                            <button className="text-[10px] uppercase tracking-wider text-stone-400 hover:text-stone-700 font-medium opacity-0 group-hover:opacity-100 transition-opacity">
                                                View →
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            ) : (
                                <p className="text-sm text-stone-400 py-2 italic">No matches</p>
                            )}
                        </div>
                    ))}
                </div>
            )}

            {/* Empty State */}
            {!isLoading && !error && logicGroups.every(l => l.results.length === 0) && (
                <div className="text-center py-12 border border-dashed border-stone-300 rounded-lg bg-stone-50/50">
                    <p className="text-stone-700 font-medium italic">"No pairings found yet."</p>
                    <p className="text-sm text-stone-500 mt-2">
                        Re-scan your library to build AI pairings.
                    </p>
                </div>
            )}

            {/* Preview Modal */}
            {previewCombo && (
                <ComboPreviewModal
                    combo={previewCombo}
                    onClose={() => setPreviewCombo(null)}
                />
            )}
        </div>
    );
};

// ============================================================================
// Logic Section (Accordion Item)
// ============================================================================

interface LogicSectionProps {
    logic: PairingByLogic;
    isExpanded: boolean;
    onToggle: () => void;
    primaryFont: FontData;
    primaryFontFamily: string;
    onPreview: (combo: FontCombo) => void;
    onSelectFont?: (font: FontData) => void;
    onFontPreview?: (font: FontData) => void;
    library: FontData[];
}

const LogicSection: React.FC<LogicSectionProps> = ({
    logic, isExpanded, onToggle, primaryFont, primaryFontFamily, onPreview, onSelectFont, onFontPreview, library
}) => {
    const hasResults = logic.results.length > 0;

    return (
        <div className={`border rounded-lg transition-all duration-200 ${isExpanded ? 'border-amber-300 bg-white shadow-sm' : 'border-stone-200 bg-stone-50/50'
            }`}>
            {/* Header */}
            <button
                onClick={onToggle}
                className="w-full flex items-center justify-between p-4 text-left hover:bg-stone-50/50 transition-colors"
            >
                <div className="flex items-center gap-3">
                    <span className={`p-2 rounded ${isExpanded ? 'bg-amber-100 text-amber-700' : 'bg-stone-100 text-stone-500'}`}>
                        {LOGIC_ICONS[logic.logic_key] || <Layers size={18} />}
                    </span>
                    <div>
                        <h5 className="font-medium text-stone-900">{logic.logic_name}</h5>
                        <p className="text-xs text-stone-500">{logic.description}</p>
                    </div>
                </div>
                <div className="flex items-center gap-2">
                    {hasResults && (
                        <span className="text-xs bg-stone-100 text-stone-600 px-2 py-0.5 rounded-full">
                            {logic.results.length} combo{logic.results.length > 1 ? 's' : ''}
                        </span>
                    )}
                    {isExpanded ? <ChevronDown size={18} /> : <ChevronRight size={18} />}
                </div>
            </button>

            {/* Content */}
            {isExpanded && (
                <div className="border-t border-stone-100 p-4 space-y-3">
                    {hasResults ? (
                        logic.results.map((combo, idx) => (
                            <ComboCard
                                key={idx}
                                combo={combo}
                                onPreview={() => onPreview(combo)}
                                onSelectFont={onSelectFont}
                                onFontPreview={onFontPreview}
                                library={library}
                            />
                        ))
                    ) : (
                        <p className="text-sm text-stone-500 text-center py-4 italic">
                            No matches for this strategy with your library.
                        </p>
                    )}
                </div>
            )}
        </div>
    );
};

// ============================================================================
// Combo Card (2-3 fonts display)
// ============================================================================

interface ComboCardProps {
    combo: FontCombo;
    onPreview: () => void;
    onSelectFont?: (font: FontData) => void;
    onFontPreview?: (font: FontData) => void;
    library: FontData[];
}

const ComboCard: React.FC<ComboCardProps> = ({ combo, onPreview, onSelectFont, onFontPreview, library }) => {
    const fontLoaders = combo.fonts.map(f => useFontLoader(f.font_id, f.file_path, true));
    const allLoaded = fontLoaders.every(l => !l.isLoading);

    return (
        <div className="group p-4 bg-stone-50 rounded-lg hover:bg-amber-50/50 transition-all">
            {/* Font List */}
            <div className="flex flex-wrap gap-2 mb-3">
                {combo.fonts.map((font, idx) => {
                    const libFont = library.find(f => f.id === font.font_id);
                    return (
                        <div
                            key={idx}
                            onClick={(e) => {
                                e.stopPropagation();
                                if (libFont && onFontPreview) {
                                    onFontPreview(libFont);
                                }
                            }}
                            className="flex items-center gap-2 px-3 py-1.5 rounded-full cursor-pointer transition-all border bg-stone-50 border-stone-200 hover:border-amber-400 hover:bg-amber-50 hover:shadow-sm"
                        >
                            <span className="text-[10px] font-bold uppercase text-stone-400">{font.role}</span>
                            <span className="text-sm font-medium text-stone-800">{font.font_name}</span>
                        </div>
                    );
                })}
            </div>

            {/* Preview */}
            <div className="space-y-2 mb-3">
                {combo.fonts.map((font, idx) => {
                    const loader = fontLoaders[idx];
                    const isHeadline = font.role === 'headline' || font.role === 'ui';
                    return (
                        <p
                            key={idx}
                            className={`${isHeadline ? 'text-2xl font-light' : 'text-base'} text-stone-700`}
                            style={{ fontFamily: loader.fontFamily }}
                        >
                            {isHeadline ? 'The quick brown fox' : 'Good typography is invisible.'}
                        </p>
                    );
                })}
            </div>

            {/* Rationale */}
            <p className="text-xs text-stone-500 italic">{combo.rationale}</p>
        </div>
    );
};

// ============================================================================
// Combo Preview Modal
// ============================================================================

interface ComboPreviewModalProps {
    combo: FontCombo;
    onClose: () => void;
}

const ComboPreviewModal: React.FC<ComboPreviewModalProps> = ({ combo, onClose }) => {
    const [editableTexts, setEditableTexts] = useState<Record<string, string>>({
        headline: 'Editorial Design',
        body: 'Good typography is invisible. It allows the reader to focus on the content without distraction.',
        caption: 'Photography by Jane Doe'
    });

    // Load fonts
    const fontLoaders = combo.fonts.map(f => useFontLoader(f.font_id, f.file_path, true));

    const getFont = (role: string) => {
        const idx = combo.fonts.findIndex(f => f.role === role ||
            (role === 'headline' && (f.role === 'ui' || f.role === 'code'))
        );
        return idx >= 0 ? fontLoaders[idx]?.fontFamily : 'inherit';
    };

    return (
        <div className="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-8 animate-in fade-in duration-200">
            <div className="bg-white rounded-2xl max-w-3xl w-full max-h-[90vh] overflow-auto shadow-2xl">
                {/* Header */}
                <div className="flex items-center justify-between p-6 border-b border-stone-100">
                    <div>
                        <h3 className="text-lg font-semibold text-stone-900">Preview Combination</h3>
                        <p className="text-sm text-stone-500">Edit text to test the pairing</p>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-full"
                    >
                        ✕
                    </button>
                </div>

                {/* Preview Area */}
                <div className="p-8 space-y-6 bg-stone-50">
                    <textarea
                        value={editableTexts.headline}
                        onChange={(e) => setEditableTexts(t => ({ ...t, headline: e.target.value }))}
                        className="w-full text-5xl font-light text-stone-900 bg-transparent border-none resize-none focus:outline-none focus:ring-0"
                        style={{ fontFamily: getFont('headline') }}
                        rows={1}
                    />
                    <textarea
                        value={editableTexts.body}
                        onChange={(e) => setEditableTexts(t => ({ ...t, body: e.target.value }))}
                        className="w-full text-lg text-stone-600 leading-relaxed bg-transparent border-none resize-none focus:outline-none focus:ring-0"
                        style={{ fontFamily: getFont('body') }}
                        rows={3}
                    />
                    <textarea
                        value={editableTexts.caption}
                        onChange={(e) => setEditableTexts(t => ({ ...t, caption: e.target.value }))}
                        className="w-full text-sm text-stone-400 bg-transparent border-none resize-none focus:outline-none focus:ring-0"
                        style={{ fontFamily: getFont('body') }}
                        rows={1}
                    />
                </div>

                {/* Font Info */}
                <div className="p-6 border-t border-stone-100 flex flex-wrap gap-4">
                    {combo.fonts.map((font, idx) => (
                        <div key={idx} className="bg-stone-50 px-4 py-2 rounded-lg">
                            <span className="text-xs text-stone-400 uppercase block">{font.role}</span>
                            <span className="text-sm font-medium text-stone-800">{font.font_name}</span>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
};
