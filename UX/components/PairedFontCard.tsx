import React from 'react';
import { ArrowRight, Loader2 } from 'lucide-react';
import { FontData } from '../types';
import { Button } from './Button';
import { useFontLoader } from '../hooks/useFontLoader';

interface PairedFontCardProps {
    primaryFont: FontData;
    primaryFontFamily: string; // The CSS font-family string for the main font
    secondaryFont: FontData;
    score: number;
    rationale: string;
}

export const PairedFontCard: React.FC<PairedFontCardProps> = ({
    primaryFont,
    primaryFontFamily,
    secondaryFont,
    score,
    rationale
}) => {
    // Load the candidate font
    const { fontFamily: secondaryFamily, isLoading } = useFontLoader(secondaryFont?.id || '', secondaryFont?.filePath);

    if (!secondaryFont) return null;

    // Determine usage based on score/hierarchy
    // If it's a very high match, it might be good for swapping roles.
    // Or generally: Display (Heading) vs Body.
    // Logic: If Primary is Display/Serif, it's likely Heading. 
    // If Secondary is Sans, it's likely Body.
    // For now, let's keep the logic simple or dynamic.

    const headingFont = score > 0.85 ? secondaryFamily : primaryFontFamily;
    const bodyFont = score > 0.85 ? primaryFontFamily : secondaryFamily;

    return (
        <div className="group border border-stone-200 hover:border-stone-400 transition-all duration-300 p-6 hover:shadow-lg hover:-translate-y-1 bg-white">
            <div className="flex justify-between items-start mb-6">
                <div className="flex gap-2">
                    <span className="text-xs font-bold bg-stone-900 text-white px-2 py-1">
                        {(score * 100).toFixed(0)}% MATCH
                    </span>
                    <span className="text-xs font-medium bg-stone-100 text-stone-600 px-2 py-1">
                        {secondaryFont.name}
                    </span>
                </div>
                {isLoading && <Loader2 size={14} className="animate-spin text-stone-400" />}
                {!isLoading && (
                    <Button variant="ghost" size="sm" className="group-hover:text-stone-900">
                        Compare <ArrowRight size={14} className="ml-1" />
                    </Button>
                )}
            </div>

            <div className={`mb-6 space-y-4 ${isLoading ? 'opacity-50 blur-[1px]' : ''}`}>
                <div>
                    <p className="text-sm text-stone-400 mb-1">Heading</p>
                    <p className="text-4xl text-stone-900 leading-tight" style={{ fontFamily: headingFont }}>
                        Editorial Design
                    </p>
                </div>
                <div>
                    <p className="text-sm text-stone-400 mb-1">Body</p>
                    <p className="text-base text-stone-600 leading-relaxed max-w-md" style={{ fontFamily: bodyFont }}>
                        Good typography is invisible. It allows the reader to focus on the content, not the form.
                        This pairing combines {primaryFont.name} and {secondaryFont.name} for optimal readability.
                    </p>
                </div>
            </div>

            <div className="pt-4 border-t border-stone-100">
                <p className="text-sm text-stone-500 italic">"{rationale}"</p>
            </div>
        </div>
    );
};
