import React, { useState, useEffect } from 'react';
import { X, Sparkles, Loader2, AlertCircle } from 'lucide-react';
import { FontData } from '../types';
import { qdrantService, SimilarFont } from '../services/qdrant';
import { Button } from './Button';

interface SimilarFontsPanelProps {
    font: FontData;
    onClose: () => void;
    onFontSelect?: (fontId: string) => void;
}

export const SimilarFontsPanel: React.FC<SimilarFontsPanelProps> = ({ font, onClose, onFontSelect }) => {
    const [similarFonts, setSimilarFonts] = useState<SimilarFont[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchSimilar = async () => {
            setIsLoading(true);
            setError(null);

            try {
                const results = await qdrantService.findSimilarFonts(font.id, 10);
                setSimilarFonts(results);
            } catch (err: any) {
                console.error('Failed to load similar fonts:', err);
                setError(err?.message || 'Failed to load similar fonts');
            } finally {
                setIsLoading(false);
            }
        };

        fetchSimilar();
    }, [font.id]);

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-stone-900/10 backdrop-blur-[1px] z-40 transition-opacity duration-300"
                onClick={onClose}
            />

            {/* Panel */}
            <div className="fixed top-0 right-0 h-full w-full md:w-[600px] bg-white shadow-2xl z-50 transform transition-transform duration-400 flex flex-col border-l border-stone-200">

                {/* Header */}
                <div className="flex items-center justify-between p-6 border-b border-stone-100 bg-white/95 backdrop-blur">
                    <div>
                        <div className="flex items-center gap-2 mb-1">
                            <Sparkles size={18} className="text-stone-600" />
                            <span className="text-xs uppercase tracking-widest text-stone-500 font-medium">
                                Similar Fonts
                            </span>
                        </div>
                        <h2 className="text-3xl font-light text-stone-900 tracking-tight">{font.name}</h2>
                    </div>
                    <Button variant="icon" onClick={onClose}>
                        <X size={20} />
                    </Button>
                </div>

                {/* Content */}
                <div className="flex-1 overflow-y-auto p-6">
                    {/* Loading State */}
                    {isLoading && (
                        <div className="flex flex-col items-center justify-center py-16">
                            <Loader2 size={48} className="animate-spin text-stone-400 mb-4" />
                            <span className="text-stone-500">Searching visual similarity...</span>
                        </div>
                    )}

                    {/* Error State */}
                    {error && !isLoading && (
                        <div className="bg-red-50 border border-red-200 rounded-lg p-6">
                            <div className="flex items-start gap-3">
                                <AlertCircle size={20} className="text-red-600 mt-0.5" />
                                <div>
                                    <h5 className="font-medium text-red-900">Unable to find similar fonts</h5>
                                    <p className="text-sm text-red-700 mt-1">{error}</p>
                                    <p className="text-xs text-red-600 mt-2">
                                        Make sure Qdrant is running and this font has been indexed with visual embeddings.
                                    </p>
                                </div>
                            </div>
                        </div>
                    )}

                    {/* Results Grid */}
                    {!isLoading && !error && similarFonts.length > 0 && (
                        <div className="space-y-4">
                            <p className="text-sm text-stone-500">
                                Found {similarFonts.length} visually similar fonts based on perceptual embeddings
                            </p>
                            <div className="grid gap-4">
                                {similarFonts.map((similar) => (
                                    <SimilarFontCard
                                        key={similar.font_id}
                                        similarFont={similar}
                                        onSelect={() => onFontSelect?.(similar.font_id)}
                                    />
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Empty State */}
                    {!isLoading && !error && similarFonts.length === 0 && (
                        <div className="text-center py-16">
                            <Sparkles size={48} className="text-stone-300 mx-auto mb-4" />
                            <p className="text-stone-500 font-medium mb-2">No similar fonts found</p>
                            <p className="text-sm text-stone-400">
                                This font may not have visual embeddings yet.
                            </p>
                        </div>
                    )}
                </div>
            </div>
        </>
    );
};

interface SimilarFontCardProps {
    similarFont: SimilarFont;
    onSelect: () => void;
}

const SimilarFontCard: React.FC<SimilarFontCardProps> = ({ similarFont, onSelect }) => {
    const similarityPercent = (similarFont.similarity_score * 100).toFixed(0);
    const similarityColor =
        similarFont.similarity_score > 0.9 ? 'bg-green-100 text-green-700' :
            similarFont.similarity_score > 0.8 ? 'bg-blue-100 text-blue-700' :
                'bg-stone-100 text-stone-700';

    return (
        <div className="group bg-white border border-stone-200 hover:border-stone-400 rounded-lg p-4 transition-all hover:shadow-md cursor-pointer"
            onClick={onSelect}
        >
            <div className="flex items-start justify-between mb-3">
                <div>
                    <h4 className="font-medium text-stone-900">{similarFont.name}</h4>
                    {similarFont.foundry && (
                        <p className="text-xs text-stone-500 mt-0.5">{similarFont.foundry}</p>
                    )}
                </div>
                <div className={`text-xs font-bold px-2 py-1 rounded ${similarityColor}`}>
                    {similarityPercent}%
                </div>
            </div>

            {/* Font Preview */}
            <div className="mb-3 p-4 bg-stone-50 rounded border border-stone-200">
                <p className="text-2xl text-stone-900" style={{ fontFamily: similarFont.name }}>
                    Aa Bb Cc Dd
                </p>
            </div>

            {/* Metrics */}
            <div className="grid grid-cols-2 gap-2 text-xs">
                <div>
                    <span className="text-stone-400">Category:</span>
                    <span className="ml-1 text-stone-700 capitalize">{similarFont.primary_class || 'Unknown'}</span>
                </div>
                <div>
                    <span className="text-stone-400">X-Height:</span>
                    <span className="ml-1 text-stone-700">{similarFont.x_height.toFixed(2)}</span>
                </div>
                <div>
                    <span className="text-stone-400">Contrast:</span>
                    <span className="ml-1 text-stone-700">{similarFont.stroke_contrast.toFixed(2)}</span>
                </div>
                <div>
                    <span className="text-stone-400">Aperture:</span>
                    <span className="ml-1 text-stone-700">{similarFont.aperture.toFixed(2)}</span>
                </div>
            </div>

            <div className="mt-3 pt-3 border-t border-stone-100">
                <button className="text-xs text-stone-500 group-hover:text-stone-900 font-medium transition-colors">
                    View Details →
                </button>
            </div>
        </div>
    );
};
