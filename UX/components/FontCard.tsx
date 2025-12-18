import React from 'react';
import { FontData } from '../types';
import { MoreHorizontal, Maximize2 } from 'lucide-react';
import { useFontLoader } from '../hooks/useFontLoader';

interface FontCardProps {
  font: FontData;
  onClick: (font: FontData) => void;
}

export const FontCard: React.FC<FontCardProps> = ({ font, onClick }) => {
  // Load font dynamically
  const { fontFamily, isLoading } = useFontLoader(font.id, font.filePath);

  // Safe metric access with defaults
  const xHeight = font.metrics?.xHeightRatio ?? 0;
  const contrast = font.metrics?.strokeContrast ?? 0;
  const upm = font.metrics?.unitsPerEm ?? 1000;

  return (
    <div
      className="group relative bg-white border border-stone-200 p-6 flex flex-col h-[320px] transition-all duration-300 hover:shadow-xl hover:shadow-stone-200/50 hover:-translate-y-1 cursor-pointer"
      onClick={() => onClick(font)}
    >
      {/* Header */}
      <div className="flex justify-between items-start mb-8">
        <div>
          <span className="inline-block text-[10px] uppercase tracking-widest text-stone-400 border border-stone-100 px-1.5 py-0.5 mb-2">
            {font.category || 'Unknown'}
          </span>
          <h3 className="font-medium text-stone-900 text-sm truncate max-w-[150px]">{font.name}</h3>
        </div>
        <button className="text-stone-300 hover:text-stone-900 transition-colors">
          <MoreHorizontal size={16} />
        </button>
      </div>

      {/* Preview */}
      <div className="flex-1 flex items-center justify-center overflow-hidden mb-6">
        {isLoading ? (
          <div className="text-stone-300 text-sm animate-pulse">Loading...</div>
        ) : (
          <span
            className="text-5xl text-stone-900 leading-none text-center whitespace-nowrap"
            style={{ fontFamily }}
          >
            Aa Bb Cc
          </span>
        )}
      </div>

      {/* Footer Metrics */}
      <div className="mt-auto border-t border-stone-100 pt-4 opacity-60 group-hover:opacity-100 transition-opacity">
        <div className="flex justify-between items-end text-xs font-mono-code text-stone-500">
          <div className="flex flex-col gap-1">
            <span>XH: {xHeight.toFixed(2)}</span>
            <span>CT: {contrast.toFixed(1)}</span>
          </div>
          <div className="flex flex-col gap-1 text-right">
            <span>{upm} UPM</span>
            <span className="flex items-center gap-1 justify-end group-hover:text-stone-900">
              View <Maximize2 size={10} />
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
