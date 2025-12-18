/**
 * Tags Editor Component
 * ========================
 * 
 * Add/remove tags on fonts
 */

import React, { useState, useRef, useEffect } from 'react';
import { Tag, Plus, X } from 'lucide-react';

interface TagsEditorProps {
    tags: string[];
    onChange: (tags: string[]) => void;
    suggestions?: string[];
    readOnly?: boolean;
}

// Common font tags for suggestions
const DEFAULT_SUGGESTIONS = [
    'Modern', 'Classic', 'Elegant', 'Bold', 'Light',
    'Editorial', 'UI', 'Display', 'Body', 'Heading',
    'Geometric', 'Humanist', 'Grotesque', 'Script', 'Decorative',
    'Web', 'Print', 'Logo', 'Branding', 'Technical',
    'Friendly', 'Professional', 'Playful', 'Serious', 'Minimal'
];

const TAG_COLORS: Record<string, string> = {
    'Modern': 'bg-blue-100 text-blue-700',
    'Classic': 'bg-amber-100 text-amber-700',
    'Elegant': 'bg-purple-100 text-purple-700',
    'Bold': 'bg-red-100 text-red-700',
    'Light': 'bg-stone-100 text-stone-600',
    'Editorial': 'bg-emerald-100 text-emerald-700',
    'UI': 'bg-cyan-100 text-cyan-700',
    'Display': 'bg-orange-100 text-orange-700',
    'Body': 'bg-green-100 text-green-700',
    'Heading': 'bg-indigo-100 text-indigo-700',
};

const getTagColor = (tag: string): string => {
    return TAG_COLORS[tag] || 'bg-stone-100 text-stone-600';
};

export const TagsEditor: React.FC<TagsEditorProps> = ({
    tags,
    onChange,
    suggestions = DEFAULT_SUGGESTIONS,
    readOnly = false
}) => {
    const [inputValue, setInputValue] = useState('');
    const [showSuggestions, setShowSuggestions] = useState(false);
    const [focusedIndex, setFocusedIndex] = useState(-1);
    const inputRef = useRef<HTMLInputElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    // Filter suggestions based on input and existing tags
    const filteredSuggestions = suggestions.filter(s =>
        !tags.includes(s) &&
        s.toLowerCase().includes(inputValue.toLowerCase())
    ).slice(0, 8);

    // Close suggestions on outside click
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
                setShowSuggestions(false);
            }
        };
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, []);

    const addTag = (tag: string) => {
        const trimmed = tag.trim();
        if (trimmed && !tags.includes(trimmed)) {
            onChange([...tags, trimmed]);
        }
        setInputValue('');
        setShowSuggestions(false);
        setFocusedIndex(-1);
        inputRef.current?.focus();
    };

    const removeTag = (tag: string) => {
        onChange(tags.filter(t => t !== tag));
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            if (focusedIndex >= 0 && filteredSuggestions[focusedIndex]) {
                addTag(filteredSuggestions[focusedIndex]);
            } else if (inputValue.trim()) {
                addTag(inputValue);
            }
        } else if (e.key === 'Backspace' && !inputValue && tags.length > 0) {
            removeTag(tags[tags.length - 1]);
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            setFocusedIndex(prev =>
                prev < filteredSuggestions.length - 1 ? prev + 1 : prev
            );
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            setFocusedIndex(prev => prev > 0 ? prev - 1 : -1);
        } else if (e.key === 'Escape') {
            setShowSuggestions(false);
            setFocusedIndex(-1);
        }
    };

    if (readOnly) {
        return (
            <div className="flex flex-wrap gap-1.5">
                {tags.length === 0 ? (
                    <span className="text-xs text-stone-400 italic">No tags</span>
                ) : (
                    tags.map(tag => (
                        <span
                            key={tag}
                            className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${getTagColor(tag)}`}
                        >
                            {tag}
                        </span>
                    ))
                )}
            </div>
        );
    }

    return (
        <div ref={containerRef} className="relative">
            <div className="flex flex-wrap items-center gap-1.5 p-2 bg-white border border-stone-200 rounded-lg focus-within:ring-2 focus-within:ring-stone-400/20 focus-within:border-stone-400">
                {/* Existing tags */}
                {tags.map(tag => (
                    <span
                        key={tag}
                        className={`inline-flex items-center gap-1 pl-2 pr-1 py-0.5 rounded-full text-xs font-medium ${getTagColor(tag)}`}
                    >
                        {tag}
                        <button
                            onClick={() => removeTag(tag)}
                            className="p-0.5 hover:bg-black/10 rounded-full transition-colors"
                        >
                            <X size={10} />
                        </button>
                    </span>
                ))}

                {/* Input */}
                <input
                    ref={inputRef}
                    type="text"
                    value={inputValue}
                    onChange={(e) => {
                        setInputValue(e.target.value);
                        setShowSuggestions(true);
                        setFocusedIndex(-1);
                    }}
                    onFocus={() => setShowSuggestions(true)}
                    onKeyDown={handleKeyDown}
                    placeholder={tags.length === 0 ? 'Add tags...' : ''}
                    className="flex-1 min-w-[80px] px-1 py-0.5 text-sm bg-transparent outline-none placeholder:text-stone-400"
                />

                {/* Add button */}
                {inputValue && (
                    <button
                        onClick={() => addTag(inputValue)}
                        className="p-1 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded transition-colors"
                    >
                        <Plus size={14} />
                    </button>
                )}
            </div>

            {/* Suggestions dropdown */}
            {showSuggestions && (inputValue || filteredSuggestions.length > 0) && (
                <div className="absolute z-10 w-full mt-1 py-1 bg-white border border-stone-200 rounded-lg shadow-lg max-h-48 overflow-auto">
                    {inputValue && !filteredSuggestions.includes(inputValue) && (
                        <button
                            onClick={() => addTag(inputValue)}
                            className="w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-stone-50"
                        >
                            <Plus size={14} className="text-stone-400" />
                            Create "{inputValue}"
                        </button>
                    )}
                    {filteredSuggestions.map((suggestion, i) => (
                        <button
                            key={suggestion}
                            onClick={() => addTag(suggestion)}
                            className={`w-full flex items-center gap-2 px-3 py-2 text-sm text-left transition-colors ${focusedIndex === i ? 'bg-stone-100' : 'hover:bg-stone-50'
                                }`}
                        >
                            <Tag size={14} className="text-stone-400" />
                            {suggestion}
                        </button>
                    ))}
                </div>
            )}
        </div>
    );
};

/**
 * Compact tag display for cards
 */
export const TagBadges: React.FC<{ tags: string[]; max?: number }> = ({ tags, max = 3 }) => {
    const displayTags = tags.slice(0, max);
    const remaining = tags.length - max;

    return (
        <div className="flex flex-wrap gap-1">
            {displayTags.map(tag => (
                <span
                    key={tag}
                    className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${getTagColor(tag)}`}
                >
                    {tag}
                </span>
            ))}
            {remaining > 0 && (
                <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-stone-100 text-stone-500">
                    +{remaining}
                </span>
            )}
        </div>
    );
};

export default TagsEditor;
