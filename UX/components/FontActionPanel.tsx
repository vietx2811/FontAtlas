import React from 'react';
import { motion } from 'framer-motion';
import { Plus, Check, ArrowRight } from 'lucide-react';
import { FontData } from '../types';
import { Button } from './Button';
import { useFontLoader } from '../hooks/useFontLoader';
import { deskService, useDeskStore } from '../services/desk';

interface FontActionPanelProps {
    font: FontData;
    onGoTo?: (font: FontData) => void;
    onClose?: () => void;
}

export const FontActionPanel: React.FC<FontActionPanelProps> = ({ font, onGoTo, onClose }) => {
    const isInDesk = useDeskStore(state => state.items.some(item => item.id === font.id));
    const { fontFamily } = useFontLoader(font.id, font.filePath, true);

    return (
        <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-3 overflow-hidden w-full"
        >
            <div className="bg-[var(--surface-3)] border border-[var(--border)] rounded-xl p-5 space-y-4 shadow-inner">
                <div
                    className="text-4xl py-6 px-4 bg-white/50 rounded-lg border border-dashed border-[var(--border)] truncate text-[var(--foreground)]"
                    style={{ fontFamily }}
                >
                    {font.name}
                </div>

                <div className="flex gap-2">
                    <Button
                        className="flex-1 gap-2"
                        onClick={() => onGoTo?.(font)}
                    >
                        <ArrowRight size={14} />
                        Go to Font
                    </Button>

                    <Button
                        variant="secondary"
                        className="flex-1 gap-2"
                        onClick={() => {
                            if (isInDesk) {
                                deskService.removeFromDesk(font.id);
                            } else {
                                deskService.addToDesk({
                                    id: font.id,
                                    name: font.name,
                                    file_path: font.filePath,
                                    primary_class: (font as any).primary_class
                                });
                            }
                        }}
                    >
                        {isInDesk ? <Check size={14} className="text-green-500" /> : <Plus size={14} />}
                        {isInDesk ? 'On Desk' : 'Add to Desk'}
                    </Button>
                </div>
            </div>
        </motion.div>
    );
};
