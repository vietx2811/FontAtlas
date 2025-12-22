import React, { useRef, useState, useEffect } from 'react';
import { FontData } from '../types';
import { ArrowRight } from 'lucide-react';
import { useFontLoader } from '../hooks/useFontLoader';

interface FontCardProps {
  font: FontData;
  onClick: (font: FontData) => void;
}

export const FontCard: React.FC<FontCardProps> = ({ font, onClick }) => {
  const cardRef = useRef<HTMLDivElement>(null);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => setIsVisible(entry.isIntersecting),
      { threshold: 0.1, rootMargin: '50px' }
    );

    if (cardRef.current) {
      observer.observe(cardRef.current);
    }

    return () => observer.disconnect();
  }, []);

  const { fontFamily, isLoading } = useFontLoader(font.id, font.filePath, isVisible);

  return (
    <div
      ref={cardRef}
      className="group relative bg-[var(--card)] border border-[var(--border)] p-5 flex flex-col h-[260px] transition-all duration-300 hover:shadow-xl hover:shadow-black/[0.03] hover:-translate-y-1 hover:border-[var(--muted)] cursor-pointer rounded-xl"
      onClick={() => onClick(font)}
    >
      {/* Header: Name + Classification */}
      <div className="flex items-start justify-between gap-2 mb-3">
        {/* Left: Name */}
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-bold text-stone-800 truncate leading-tight">{font.name}</h3>
          <p className="text-[9px] text-stone-400 truncate uppercase tracking-widest mt-0.5">{font.family}</p>
        </div>

        {/* Right: Classification Badge - Smaller */}
        {(font as any).primary_class && (
          <span className="shrink-0 text-[8px] uppercase tracking-wider font-bold text-amber-700 bg-amber-50 border border-amber-200 px-1.5 py-0.5 rounded-full">
            {(font as any).primary_class}
          </span>
        )}
      </div>

      {/* Preview */}
      <div className="flex-1 flex items-center justify-center overflow-hidden">
        {!isVisible ? (
          <div className="text-[var(--border)] text-sm">Preview</div>
        ) : isLoading ? (
          <div className="text-[var(--border)] text-sm animate-pulse">Loading...</div>
        ) : (
          <span
            className="text-5xl text-[var(--foreground)] leading-none text-center"
            style={{ fontFamily }}
          >
            Aa Bb Cc
          </span>
        )}
      </div>
    </div>
  );
};
