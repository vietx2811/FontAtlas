/**
 * Font Family Card Component
 * Displays a font family with all its weight variants
 * On hover: dynamically transitions between weights based on mouse X position
 */

import React, { useState, useRef } from 'react';
import { FontFamily } from '../types';

interface FontFamilyCardProps {
    family: FontFamily;
    onSelect: (fontId: string) => void;
    isSelected?: boolean;
}

export const FontFamilyCard: React.FC<FontFamilyCardProps> = ({
    family,
    onSelect,
    isSelected = false
}) => {
    const [activeVariantIndex, setActiveVariantIndex] = useState(0);
    const cardRef = useRef<HTMLDivElement>(null);

    // Sort variants by weight_class for smooth transition
    const sortedVariants = [...family.variants].sort((a, b) => a.weight_class - b.weight_class);
    const activeVariant = sortedVariants[activeVariantIndex] || sortedVariants[0];

    const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
        if (!cardRef.current) return;

        const rect = cardRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const percentage = x / rect.width;

        // Map mouse X position to variant index
        const index = Math.floor(percentage * sortedVariants.length);
        const clampedIndex = Math.max(0, Math.min(sortedVariants.length - 1, index));

        setActiveVariantIndex(clampedIndex);
    };

    const handleMouseLeave = () => {
        // Reset to representative variant
        const repIndex = sortedVariants.findIndex(v => v.id === family.representative_id);
        setActiveVariantIndex(repIndex >= 0 ? repIndex : 0);
    };

    const handleClick = () => {
        onSelect(activeVariant.id);
    };

    // Get weight name for display
    const getWeightName = (weightClass: number): string => {
        const weights: Record<number, string> = {
            100: 'Thin',
            200: 'Extra Light',
            300: 'Light',
            400: 'Regular',
            500: 'Medium',
            600: 'Semi Bold',
            700: 'Bold',
            800: 'Extra Bold',
            900: 'Black'
        };
        return weights[weightClass] || `${weightClass}`;
    };

    return (
        <div
            ref={cardRef}
            onMouseMove={handleMouseMove}
            onMouseLeave={handleMouseLeave}
            onClick={handleClick}
            className={`
        group relative cursor-pointer rounded-xl border-2 p-6
        transition-all duration-200 hover:shadow-lg
        ${isSelected
                    ? 'border-stone-800 bg-stone-50'
                    : 'border-stone-200 bg-white hover:border-stone-400'
                }
      `}
        >
            {/* Family Name */}
            <div className="mb-4">
                <h3 className="text-lg font-semibold text-stone-900">
                    {family.family_name}
                </h3>
                <p className="text-sm text-stone-500">
                    {sortedVariants.length} {sortedVariants.length === 1 ? 'variant' : 'variants'}
                </p>
            </div>

            {/* Preview Text with Dynamic Weight */}
            <div className="mb-4 overflow-hidden">
                <p
                    className="text-4xl text-stone-900 transition-all duration-150"
                    style={{
                        fontFamily: `"${family.family_name}"`,
                        fontWeight: activeVariant.weight_class,
                        fontStyle: activeVariant.is_italic ? 'italic' : 'normal'
                    }}
                >
                    Aa
                </p>
            </div>

            {/* Active Variant Info */}
            <div className="flex items-center justify-between text-sm">
                <span className="font-medium text-stone-700">
                    {getWeightName(activeVariant.weight_class)}
                    {activeVariant.is_italic && ' Italic'}
                </span>
                <span className="text-stone-400">
                    {activeVariant.weight_class}
                </span>
            </div>

            {/* Variant Indicator Dots */}
            <div className="mt-3 flex gap-1">
                {sortedVariants.map((variant, index) => (
                    <div
                        key={variant.id}
                        className={`h-1 flex-1 rounded-full transition-colors duration-150 ${index === activeVariantIndex
                                ? 'bg-stone-800'
                                : 'bg-stone-200'
                            }`}
                    />
                ))}
            </div>

            {/* Hover Instruction (subtle) */}
            <div className="absolute bottom-2 right-2 opacity-0 transition-opacity group-hover:opacity-100">
                <p className="text-xs text-stone-400">
                    Move mouse to explore weights
                </p>
            </div>
        </div>
    );
};
