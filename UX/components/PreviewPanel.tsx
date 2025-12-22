import React from 'react';
import { motion } from 'framer-motion';
import { X, ArrowRight, Plus, Check } from 'lucide-react';
import { FontData } from '../types';
import { Button } from './Button';
import { useFontLoader } from '../hooks/useFontLoader';
import { deskService, useDeskStore } from '../services/desk';

interface PreviewPanelProps {
    font: FontData;
    onClose: () => void;
    onGoToFont: (font: FontData) => void;
}

export const PreviewPanel: React.FC<PreviewPanelProps> = ({ font, onClose, onGoToFont }) => {
    const { fontFamily, isLoading } = useFontLoader(font.id, font.filePath, true);
    const isInDesk = useDeskStore(state => state.items.some(item => item.id === font.id));

    const handleAddToDesk = () => {
        if (isInDesk) {
            deskService.removeFromDesk(font.id);
        } else {
            deskService.addToDesk({
                id: font.id,
                name: font.name,
                file_path: font.filePath || '',
                primary_class: font.primary_class
            });
        }
    };

    // Debug: Log when panel renders
    console.log('PreviewPanel rendering for font:', font.name);

    return (
        <motion.div
            initial={{ x: '100%' }}
            animate={{ x: 0 }}
            exit={{ x: '100%' }}
            transition={{ type: 'spring', damping: 30, stiffness: 300 }}
            className="fixed right-0 top-0 h-screen w-[480px] bg-[var(--background)] border-l border-[var(--border)] shadow-2xl z-[60] flex flex-col"
        >
            {/* Header */}
            <div className="flex items-center justify-between p-6 border-b border-[var(--border)]">
                <div className="flex-1 min-w-0">
                    <h3 className="text-sm font-bold text-[var(--foreground)] truncate">{font.name}</h3>
                    <p className="text-xs text-[var(--muted)] mt-0.5">{font.family}</p>
                </div>
                <button
                    onClick={onClose}
                    className="ml-4 p-2 hover:bg-[var(--surface-2)] rounded-lg transition-colors"
                >
                    <X size={18} className="text-[var(--muted)]" />
                </button>
            </div>

            {/* Preview Content */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6">
                {/* Large Specimen */}
                <div className="space-y-4">
                    <h4 className="text-[10px] uppercase font-bold text-[var(--muted)] tracking-widest">Specimen</h4>
                    {isLoading ? (
                        <div className="h-32 flex items-center justify-center bg-[var(--surface-2)] rounded-xl border border-[var(--border)]">
                            <div className="w-6 h-6 rounded-full border-2 border-[var(--muted)] border-t-transparent animate-spin" />
                        </div>
                    ) : (
                        <div className="space-y-3">
                            <div
                                className="text-6xl leading-tight text-[var(--foreground)] p-6 bg-white/50 rounded-xl border border-dashed border-[var(--border)]"
                                style={{ fontFamily }}
                            >
                                {font.name}
                            </div>
                            <div
                                className="text-3xl leading-relaxed text-[var(--foreground)] p-4 bg-white/30 rounded-lg"
                                style={{ fontFamily }}
                            >
                                The quick brown fox jumps over the lazy dog
                            </div>
                            <div
                                className="text-lg leading-relaxed text-[var(--foreground)] p-4 bg-white/20 rounded-lg"
                                style={{ fontFamily }}
                            >
                                ABCDEFGHIJKLMNOPQRSTUVWXYZ<br />
                                abcdefghijklmnopqrstuvwxyz<br />
                                0123456789 !@#$%^&*()
                            </div>
                        </div>
                    )}
                </div>

                {/* Classification */}
                {font.primary_class && (
                    <div className="space-y-2">
                        <h4 className="text-[10px] uppercase font-bold text-[var(--muted)] tracking-widest">Classification</h4>
                        <div className="flex flex-wrap gap-2">
                            <span className="px-3 py-1.5 bg-[var(--surface-2)] border border-[var(--border)] rounded-full text-xs font-medium text-[var(--foreground)]">
                                {font.primary_class}
                            </span>
                            {font.serif_subtype && (
                                <span className="px-3 py-1.5 bg-amber-50 border border-amber-200 rounded-full text-xs font-medium text-amber-900">
                                    {font.serif_subtype}
                                </span>
                            )}
                            {font.sans_subtype && (
                                <span className="px-3 py-1.5 bg-blue-50 border border-blue-200 rounded-full text-xs font-medium text-blue-900">
                                    {font.sans_subtype}
                                </span>
                            )}
                        </div>
                    </div>
                )}

                {/* Metrics Preview */}
                <div className="space-y-2">
                    <h4 className="text-[10px] uppercase font-bold text-[var(--muted)] tracking-widest">Key Metrics</h4>
                    <div className="grid grid-cols-2 gap-3">
                        <div className="p-3 bg-[var(--surface-2)] rounded-lg border border-[var(--border)]">
                            <div className="text-[10px] uppercase font-bold text-[var(--muted)] mb-1">x-Height</div>
                            <div className="text-lg font-bold text-[var(--foreground)]">{(font.metrics.xHeightRatio * 100).toFixed(0)}%</div>
                        </div>
                        <div className="p-3 bg-[var(--surface-2)] rounded-lg border border-[var(--border)]">
                            <div className="text-[10px] uppercase font-bold text-[var(--muted)] mb-1">Contrast</div>
                            <div className="text-lg font-bold text-[var(--foreground)]">{(font.metrics.strokeContrast * 100).toFixed(0)}%</div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Action Footer */}
            <div className="p-6 border-t border-[var(--border)] space-y-3">
                <Button
                    className="w-full gap-2"
                    onClick={() => onGoToFont(font)}
                >
                    <ArrowRight size={14} />
                    Open Full Details
                </Button>

                <Button
                    variant="secondary"
                    className="w-full gap-2"
                    onClick={handleAddToDesk}
                >
                    {isInDesk ? <Check size={14} className="text-green-500" /> : <Plus size={14} />}
                    {isInDesk ? 'On Desk' : 'Add to Desk'}
                </Button>
            </div>
        </motion.div>
    );
};
