import React, { useState, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LayoutGrid, Settings, FolderOpen, Database, BarChart3, Info, Plus, Folder, ChevronRight, Cloud, HardDrive } from 'lucide-react';
import { FontCategory } from '../types';
import { CollectionsManager, type Collection } from './CollectionsManager';
import { CreditsPage } from './CreditsPage';
import { PreferencesPanel } from './PreferencesPanel';
// import { DirectoriesPanel } from './DirectoriesPanel'; // Deprecated
import { useSettings } from '../hooks/useSettings';
import { useDirectories } from '../hooks/useDirectories';
import { clearLibrary } from '../services/library';
import { getSourcesWithCounts, type Source } from '../services/sources';
import { Loader2, Trash2, RefreshCw } from 'lucide-react';

interface SidebarProps {
    selectedCategory: FontCategory | 'All';
    onSelectCategory: (cat: FontCategory | 'All') => void;
    fontCount: number;
    onSelectCollection?: (collection: Collection | null) => void;
    selectedCollectionId?: string | null;
    onScanDirectory?: (path: string) => Promise<number>;
    selectedTags?: string[];
    onSelectedTagsChange?: (tags: string[]) => void;
    fonts?: Array<{ secondary_tags?: string | null }>; // For extracting available tags
}

export const Sidebar: React.FC<SidebarProps> = ({
    selectedCategory,
    onSelectCategory,
    fontCount,
    onSelectCollection,
    selectedCollectionId,
    onScanDirectory,
    selectedTags = [],
    onSelectedTagsChange,
    fonts = []
}) => {
    const [showCredits, setShowCredits] = useState(false);
    const [showPreferences, setShowPreferences] = useState(false);
    const { settings } = useSettings();

    // Directory management
    const {
        directories,
        addDirectory,
        removeDirectory,
        rescanDirectory,
        scanningPath,
        isClearing
    } = useDirectories(onScanDirectory, clearLibrary);

    // Toggle state for directories section listing
    const [isDirsExpanded, setIsDirsExpanded] = useState(true);

    // Sources state
    const [sources, setSources] = useState<Source[]>([]);
    const [selectedSourceId, setSelectedSourceId] = useState<string | null>(null);
    // selectedTags now comes from props (App.tsx manages it)

    // Extract available tags from fonts' secondary_tags (real data from database)
    const availableTags = React.useMemo(() => {
        const tagSet = new Set<string>();
        fonts.forEach(font => {
            if (font.secondary_tags) {
                try {
                    // secondary_tags is a JSON string like ["tag1","tag2"]
                    const tags = typeof font.secondary_tags === 'string'
                        ? JSON.parse(font.secondary_tags)
                        : font.secondary_tags;
                    if (Array.isArray(tags)) {
                        tags.forEach((tag: string) => tagSet.add(tag));
                    }
                } catch (e) {
                    // Skip malformed JSON
                }
            }
        });
        return Array.from(tagSet).sort();
    }, [fonts]);

    // Use extracted tags if available, fallback to common tags for empty library
    const popularTags = availableTags.length > 0
        ? availableTags
        : ['clean', 'modern', 'luxury', 'elegant', 'classic', 'warm', 'tech', 'editorial'];

    // Load sources on mount
    React.useEffect(() => {
        const loadSources = async () => {
            try {
                const sourcesData = await getSourcesWithCounts();
                setSources(sourcesData);
            } catch (error) {
                console.error('Failed to load sources:', error);
            }
        };
        loadSources();

        // Reload when directories change
    }, [directories]);

    return (
        <>
            <aside className="w-64 border-r border-stone-200 bg-white h-screen fixed left-0 top-0 flex flex-col z-30 hidden md:flex">
                {/* Drag Region for macOS Traffic Lights */}
                <div
                    className="absolute top-0 left-0 w-full h-10 z-50 bg-transparent cursor-default"
                    data-tauri-drag-region="true"
                />

                {/* Logo Header - Added top padding to clear traffic lights if needed, or just align nicely */}
                <div className="flex items-center px-6 pt-8 pb-3 shrink-0 relative z-40">
                    <div className="w-9 h-9 bg-gradient-to-br from-amber-400 to-orange-500 rounded-lg mr-3 flex items-center justify-center">
                        <span className="text-white text-sm font-bold">F</span>
                    </div>
                    <span className="font-semibold text-lg text-stone-900 tracking-tight">FontAtlas</span>
                </div>

                {/* Main Nav */}
                <div className="flex-1 overflow-y-auto sidebar-scroll pt-2 pb-4 px-4 space-y-6">

                    {/* Library Section */}
                    <div>
                        <h3 className="text-[11px] uppercase tracking-widest text-stone-400 font-semibold mb-3 px-2">Library</h3>
                        <ul className="space-y-1">
                            <li>
                                <button
                                    onClick={() => {
                                        onSelectCategory('All');
                                        onSelectCollection?.(null);
                                    }}
                                    className={`w-full flex items-center justify-between px-3 py-2.5 text-sm rounded-lg transition-colors ${selectedCategory === 'All' && !selectedCollectionId
                                        ? 'bg-stone-900 text-white font-medium'
                                        : 'text-stone-600 hover:bg-stone-100'
                                        }`}
                                >
                                    <span className="flex items-center gap-3"><LayoutGrid size={16} /> All Fonts</span>
                                    <span className="text-xs opacity-60">{fontCount}</span>
                                </button>
                            </li>
                        </ul>
                    </div>

                    {/* Directories Section */}
                    <div>
                        <div className="flex items-center justify-between mb-2 px-2 group">
                            <button
                                onClick={() => setIsDirsExpanded(!isDirsExpanded)}
                                className="flex items-center gap-2 text-[11px] uppercase tracking-widest text-stone-400 font-semibold hover:text-stone-600 transition-colors"
                            >
                                <ChevronRight
                                    size={14}
                                    className={`transition-transform duration-200 ${isDirsExpanded ? 'rotate-90' : ''}`}
                                />
                                Directories
                            </button>
                            <button
                                onClick={addDirectory}
                                className="p-1 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded transition-colors"
                                title="Add Directory to Watch"
                            >
                                <Plus size={14} />
                            </button>
                        </div>

                        {isDirsExpanded && (
                            <ul className="space-y-1">
                                {directories.length > 0 ? (
                                    directories.map(dir => (
                                        <li key={dir.path} className="group relative">
                                            <div className="w-full flex items-center justify-between px-3 py-2 text-sm text-stone-600 hover:bg-stone-50 rounded-lg transition-colors group-hover:pr-14">
                                                <span className="flex items-center gap-3 min-w-0" title={dir.path}>
                                                    <Folder size={16} className="shrink-0 text-amber-500/70" />
                                                    <span className="truncate">{dir.path.split('/').pop()}</span>
                                                </span>
                                                <span className="text-xs text-stone-300 font-mono group-hover:hidden">
                                                    {dir.fontCount}
                                                </span>
                                            </div>

                                            {/* Hover Actions */}
                                            <div className="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity bg-stone-50 pl-2">
                                                <button
                                                    onClick={() => rescanDirectory(dir)}
                                                    className={`p-1.5 text-stone-400 hover:text-stone-600 rounded-md hover:bg-stone-200 ${scanningPath === dir.path ? 'animate-spin text-amber-500' : ''}`}
                                                    title="Rescan"
                                                >
                                                    {scanningPath === dir.path ? <Loader2 size={12} /> : <RefreshCw size={12} />}
                                                </button>
                                                <button
                                                    onClick={() => removeDirectory(dir.path)}
                                                    className="p-1.5 text-stone-400 hover:text-red-500 rounded-md hover:bg-red-50"
                                                    title="Remove"
                                                >
                                                    <Trash2 size={12} />
                                                </button>
                                            </div>
                                        </li>
                                    ))
                                ) : (
                                    <li className="px-3 py-4 text-center border-2 border-dashed border-stone-100 rounded-lg">
                                        <button
                                            onClick={addDirectory}
                                            className="text-xs text-stone-400 hover:text-stone-600 flex flex-col items-center gap-1 w-full"
                                        >
                                            <Plus size={16} />
                                            <span>Add Folder</span>
                                        </button>
                                    </li>
                                )}
                            </ul>
                        )}
                    </div>

                    {/* Collections Section */}
                    <div>
                        <div className="flex items-center justify-between mb-3 px-2">
                            <h3 className="text-[11px] uppercase tracking-widest text-stone-400 font-semibold">Collections</h3>
                            <button className="p-1 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded transition-colors">
                                <Plus size={14} />
                            </button>
                        </div>
                        <ul className="space-y-1">
                            <li>
                                <button className="w-full flex items-center justify-between px-3 py-2 text-sm text-stone-600 hover:bg-stone-50 rounded-lg transition-colors">
                                    <span className="flex items-center gap-3">
                                        <span className="w-3 h-3 rounded-full bg-red-400"></span>
                                        Favorites
                                    </span>
                                    <span className="text-xs text-stone-400">12</span>
                                </button>
                            </li>
                            <li>
                                <button className="w-full flex items-center justify-between px-3 py-2 text-sm text-stone-600 hover:bg-stone-50 rounded-lg transition-colors">
                                    <span className="flex items-center gap-3">
                                        <span className="w-3 h-3 rounded-full bg-teal-400"></span>
                                        Work Projects
                                    </span>
                                    <span className="text-xs text-stone-400">8</span>
                                </button>
                            </li>
                            <li>
                                <button className="w-full flex items-center justify-between px-3 py-2 text-sm text-stone-600 hover:bg-stone-50 rounded-lg transition-colors">
                                    <span className="flex items-center gap-3">
                                        <span className="w-3 h-3 rounded-full bg-blue-400"></span>
                                        Editorial
                                    </span>
                                    <span className="text-xs text-stone-400">5</span>
                                </button>
                            </li>
                        </ul>
                    </div>

                    {/* Sources Section */}
                    <div>
                        <div className="flex items-center justify-between mb-3 px-2">
                            <h3 className="text-[11px] uppercase tracking-widest text-stone-400 font-semibold">Sources</h3>
                        </div>
                        <ul className="space-y-1">
                            {sources.length === 0 ? (
                                <li className="px-3 py-2 text-xs text-stone-400 text-center">
                                    No sources yet
                                </li>
                            ) : (
                                sources.map(source => (
                                    <li key={source.id}>
                                        <button
                                            onClick={() => setSelectedSourceId(
                                                selectedSourceId === source.id ? null : source.id
                                            )}
                                            className={`w-full flex items-center justify-between px-3 py-2 text-sm rounded-lg transition-colors ${selectedSourceId === source.id
                                                ? 'bg-stone-100 text-stone-900'
                                                : 'text-stone-600 hover:bg-stone-50'
                                                }`}
                                        >
                                            <span className="flex items-center gap-3 truncate">
                                                {source.source_type === 'directory' ? (
                                                    <HardDrive size={16} className="flex-shrink-0" />
                                                ) : (
                                                    <Cloud size={16} className="flex-shrink-0" />
                                                )}
                                                <span className="truncate" title={source.name}>
                                                    {source.name}
                                                </span>
                                            </span>
                                            <span className="text-xs text-stone-400 ml-2">
                                                {source.font_count || 0}
                                            </span>
                                        </button>
                                    </li>
                                ))
                            )}
                        </ul>
                    </div>

                    {/* Tag Filters Section */}
                    <div>
                        <div className="flex items-center justify-between mb-3 px-2">
                            <h3 className="text-[11px] uppercase tracking-widest text-stone-400 font-semibold">Filter by Tags</h3>
                            {selectedTags.length > 0 && (
                                <button
                                    onClick={() => onSelectedTagsChange?.(([]))}
                                    className="text-[10px] text-stone-400 hover:text-stone-600 transition-colors"
                                >
                                    Clear
                                </button>
                            )}
                        </div>
                        <div className="flex flex-wrap gap-1.5 px-2">
                            {popularTags.map(tag => {
                                const isActive = selectedTags.includes(tag);
                                return (
                                    <button
                                        key={tag}
                                        onClick={() => {
                                            onSelectedTagsChange?.(
                                                isActive
                                                    ? selectedTags.filter(t => t !== tag)
                                                    : [...selectedTags, tag]
                                            );
                                        }}
                                        className={`px-2 py-0.5 text-xs rounded-full transition-all capitalize ${isActive
                                            ? 'bg-amber-500/20 text-amber-600 border border-amber-500/30 font-medium'
                                            : 'bg-stone-100 text-stone-500 border border-transparent hover:bg-stone-200 hover:border-stone-300'
                                            }`}
                                    >
                                        {tag}
                                    </button>
                                );
                            })}
                        </div>
                        {selectedTags.length > 0 && (
                            <div className="mt-2 px-2 text-xs text-stone-400">
                                {selectedTags.length} tag{selectedTags.length > 1 ? 's' : ''} active
                            </div>
                        )}
                    </div>

                </div>

                {/* Footer */}
                <div className="p-4 border-t border-stone-100 space-y-3">
                    {/* Qdrant Status */}
                    <button
                        onClick={() => setShowPreferences(true)}
                        className="w-full flex items-center justify-between px-3 py-2 text-sm text-stone-600 hover:bg-stone-50 rounded-lg transition-colors"
                    >
                        <span className="flex items-center gap-2">
                            {settings.qdrantMode === 'local' ? <HardDrive size={15} /> : <Cloud size={15} />}
                            <span>Qdrant</span>
                        </span>
                        <span className="flex items-center gap-1.5">
                            <span className="text-[10px] text-stone-400">
                                {settings.qdrantMode === 'local' ? 'Local' : 'Remote'}
                            </span>
                            <span className="w-2 h-2 rounded-full bg-green-500"></span>
                        </span>
                    </button>

                    {/* Action Buttons */}
                    <div className="flex items-center justify-between gap-2">
                        <button className="flex items-center gap-2 px-3 py-2 text-sm text-stone-500 hover:bg-stone-50 rounded-lg transition-colors">
                            <BarChart3 size={16} /> Analytics
                        </button>
                        <button
                            onClick={() => setShowPreferences(true)}
                            className="flex items-center gap-2 px-3 py-2 text-sm text-stone-500 hover:bg-stone-50 rounded-lg transition-colors"
                        >
                            <Settings size={16} />
                        </button>
                        <button
                            onClick={() => setShowCredits(true)}
                            className="flex items-center gap-2 px-3 py-2 text-sm text-stone-500 hover:bg-stone-50 rounded-lg transition-colors"
                        >
                            <Info size={16} />
                        </button>
                    </div>
                </div>
            </aside>

            {/* Modals */}
            {showCredits && <CreditsPage onClose={() => setShowCredits(false)} />}
            <PreferencesPanel isOpen={showPreferences} onClose={() => setShowPreferences(false)} />
        </>
    );
};
