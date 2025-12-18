/**
 * Virtualized Font Grid Component
 * ================================
 * 
 * Efficiently renders large font collections using intersection observer
 * for lazy loading instead of react-window (simpler approach).
 */

import React, { useMemo, useRef, useEffect, useState, useCallback } from 'react';
import { FontCard } from './FontCard';
import { FontData } from '../types';

interface VirtualizedFontGridProps {
    fonts: FontData[];
    selectedFontId?: string;
    onFontSelect: (font: FontData) => void;
    isLoading?: boolean;
}

// Number of fonts to render per batch
const BATCH_SIZE = 20;

export const VirtualizedFontGrid: React.FC<VirtualizedFontGridProps> = ({
    fonts,
    selectedFontId,
    onFontSelect,
    isLoading = false
}) => {
    const [visibleCount, setVisibleCount] = useState(BATCH_SIZE);
    const loadMoreRef = useRef<HTMLDivElement>(null);

    // Reset visible count when fonts change
    useEffect(() => {
        setVisibleCount(BATCH_SIZE);
    }, [fonts]);

    // Intersection observer for infinite scroll
    useEffect(() => {
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting && visibleCount < fonts.length) {
                    setVisibleCount(prev => Math.min(prev + BATCH_SIZE, fonts.length));
                }
            },
            { threshold: 0.1, rootMargin: '100px' }
        );

        if (loadMoreRef.current) {
            observer.observe(loadMoreRef.current);
        }

        return () => observer.disconnect();
    }, [visibleCount, fonts.length]);

    // Visible fonts
    const visibleFonts = useMemo(() => {
        return fonts.slice(0, visibleCount);
    }, [fonts, visibleCount]);

    console.log('VirtualizedFontGrid: fonts.length=', fonts.length, 'isLoading=', isLoading, 'visibleCount=', visibleCount);

    if (isLoading) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-stone-400">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-stone-400 mb-4" />
                <p>Loading fonts...</p>
            </div>
        );
    }

    if (fonts.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center h-full py-20 text-stone-400 text-center animate-in fade-in zoom-in-95 duration-500">
                <div className="w-16 h-16 mb-6 rounded-2xl bg-stone-100 flex items-center justify-center">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="32"
                        height="32"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        className="text-stone-400"
                    >
                        <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                        <path d="M12 10v6" />
                        <path d="M9 13h6" />
                    </svg>
                </div>
                <p className="mb-2 text-xl font-medium text-stone-600 tracking-tight">Your Library is Empty</p>
                <p className="text-sm text-stone-500 max-w-xs mx-auto leading-relaxed">
                    Add font directories from the sidebar to verify parsing and start building your collection.
                </p>
            </div>
        );
    }

    return (
        <div className="w-full h-full">
            {/* Font count indicator */}
            <div className="pb-3 text-sm text-stone-500">
                Showing {visibleCount} of {fonts.length} fonts
                {fonts.length > 50 && (
                    <span className="ml-2 text-xs text-stone-400">
                        (scroll for more)
                    </span>
                )}
            </div>

            {/* Font Grid */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
                {visibleFonts.map(font => (
                    <FontCard
                        key={font.id}
                        font={font}
                        onClick={() => onFontSelect(font)}
                    />
                ))}
            </div>

            {/* Load more trigger */}
            {visibleCount < fonts.length && (
                <div
                    ref={loadMoreRef}
                    className="flex justify-center py-8 text-stone-400"
                >
                    <div className="animate-pulse">Loading more...</div>
                </div>
            )}
        </div>
    );
};
