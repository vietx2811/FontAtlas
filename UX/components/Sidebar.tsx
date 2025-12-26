import React, { useState, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LayoutGrid, Settings, FolderOpen, Database, BarChart3, Info, Plus, Folder, ChevronRight, Cloud, HardDrive } from 'lucide-react';
import { FontCategory } from '../types';
import { CollectionsManager, type Collection } from './CollectionsManager';
import { CreditsPage } from './CreditsPage';
import { PreferencesPanel } from './PreferencesPanel';
import { useSettings } from '../hooks/useSettings';
import { useDirectories } from '../hooks/useDirectories';
import { clearLibrary } from '../services/library';
import { getSourcesWithCounts, type Source } from '../services/sources';
import { Loader2, Trash2, RefreshCw, Heart } from 'lucide-react';

interface SidebarProps {
    selectedCategory: FontCategory | 'All';
    onSelectCategory: (cat: FontCategory | 'All') => void;
    fontCount: number;
    onSelectCollection?: (collection: Collection | null) => void;
    selectedCollectionId?: string | null;
    onScanDirectory?: (path: string) => Promise<number>;
    showFavoritesOnly?: boolean;
    onShowFavoritesChange?: (show: boolean) => void;
    selectedDirectory?: string | null;
    onSelectDirectory?: (path: string | null) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
    selectedCategory,
    onSelectCategory,
    fontCount,
    onSelectCollection,
    selectedCollectionId,
    onScanDirectory,
    showFavoritesOnly = false,
    onShowFavoritesChange,
    selectedDirectory = null,
    onSelectDirectory
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
        scanningPath
    } = useDirectories(onScanDirectory, clearLibrary);

    // Toggle state for directories section listing
    const [isDirsExpanded, setIsDirsExpanded] = useState(true);

    // Sources state
    const [sources, setSources] = useState<Source[]>([]);
    const [selectedSourceId, setSelectedSourceId] = useState<string | null>(null);

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
    }, [directories]);

    return (
        <>
            <aside className="w-64 border-r border-[var(--border)] bg-[var(--card)] h-screen fixed left-0 top-0 flex flex-col z-30 hidden md:flex">
                {/* Drag Region for macOS Traffic Lights */}
                <div
                    className="absolute top-0 left-0 w-full h-10 z-50 bg-transparent cursor-default"
                    data-tauri-drag-region="true"
                />

                {/* Logo Header */}
                {/* <div className="flex items-center px-10 pt-10 pb-3 shrink-0 relative z-40">
                    <div className="w-9 h-9 bg-gradient-to-br from-amber-400 to-orange-500 rounded-lg mr-3 flex items-center justify-center">
                        <span className="text-white text-sm font-bold shadow-sm">F</span>
                    </div>
                    <span className="font-semibold text-lg text-[var(--foreground)] tracking-tight">FontAtlas</span>
                </div> */}

                {/* Main Nav */}
                <div className="flex-1 overflow-y-auto sidebar-scroll pt-2 pb-4 px-4 space-y-6">

                    {/* Library Section */}
                    <div>
                        <h3 className="text-[11px] uppercase tracking-widest text-[var(--muted)] font-bold mb-3 px-2 opacity-50">Library</h3>
                        <ul className="space-y-1">
                            <li>
                                <button
                                    onClick={() => {
                                        onSelectCategory('All');
                                        onSelectCollection?.(null);
                                        onShowFavoritesChange?.(false);
                                        onSelectDirectory?.(null);
                                    }}
                                    className={`w-full flex items-center justify-between px-3 py-2.5 text-sm rounded-lg transition-colors ${selectedCategory === 'All' && !selectedCollectionId && !showFavoritesOnly && !selectedDirectory
                                        ? 'bg-[var(--foreground)] text-[var(--background)] font-medium'
                                        : 'text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)]'
                                        }`}
                                >
                                    <span className="flex items-center gap-3"><LayoutGrid size={16} /> All Fonts</span>
                                    <span className={`text-[10px] font-bold ${selectedCategory === 'All' ? 'opacity-80' : 'opacity-40'}`}>{fontCount}</span>
                                </button>
                            </li>
                            <li>
                                <button
                                    onClick={() => {
                                        onShowFavoritesChange?.(true);
                                        onSelectDirectory?.(null);
                                    }}
                                    className={`w-full flex items-center justify-between px-3 py-2.5 text-sm rounded-lg transition-colors ${showFavoritesOnly
                                        ? 'bg-[var(--foreground)] text-[var(--background)] font-medium'
                                        : 'text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)]'
                                        }`}
                                >
                                    <span className="flex items-center gap-3">
                                        <Heart size={16} className={showFavoritesOnly ? "fill-[var(--background)]" : ""} />
                                        Favorites
                                    </span>
                                </button>
                            </li>
                        </ul>
                    </div>

                    {/* Directories Section */}
                    <div>
                        <div className="flex items-center justify-between mb-2 px-2 group">
                            <button
                                onClick={() => setIsDirsExpanded(!isDirsExpanded)}
                                className="flex items-center gap-2 text-[11px] uppercase tracking-widest text-[var(--muted)] font-bold opacity-50 hover:opacity-100 transition-opacity"
                            >
                                <ChevronRight
                                    size={14}
                                    className={`transition-transform duration-200 ${isDirsExpanded ? 'rotate-90' : ''}`}
                                />
                                Directories
                            </button>
                            <button
                                onClick={addDirectory}
                                className="p-1 text-[var(--muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-3)] rounded transition-colors"
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
                                            <button
                                                onClick={() => {
                                                    onSelectDirectory?.(selectedDirectory === dir.path ? null : dir.path);
                                                    onShowFavoritesChange?.(false);
                                                }}
                                                className={`w-full flex items-center justify-between px-3 py-2 text-sm rounded-lg transition-colors group-hover:pr-14 ${selectedDirectory === dir.path
                                                    ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
                                                    : 'text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)]'
                                                    }`}
                                            >
                                                <span className="flex items-center gap-3 min-w-0" title={dir.path}>
                                                    <Folder size={16} className={`shrink-0 ${selectedDirectory === dir.path ? 'text-[var(--foreground)]' : 'text-amber-500/70'}`} />
                                                    <span className="truncate">{dir.path.split('/').pop()}</span>
                                                </span>
                                                <span className="text-[10px] font-bold opacity-40 group-hover:hidden">
                                                    {dir.fontCount}
                                                </span>
                                            </button>

                                            {/* Hover Actions */}
                                            <div className="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity bg-[var(--surface-3)] rounded-md p-0.5 shadow-sm">
                                                <button
                                                    onClick={() => rescanDirectory(dir)}
                                                    className={`p-1.5 text-[var(--muted)] hover:text-[var(--foreground)] rounded-md ${scanningPath === dir.path ? 'animate-spin text-amber-500' : ''}`}
                                                    title="Rescan"
                                                >
                                                    {scanningPath === dir.path ? <Loader2 size={12} /> : <RefreshCw size={12} />}
                                                </button>
                                                <button
                                                    onClick={() => removeDirectory(dir.path)}
                                                    className="p-1.5 text-[var(--muted)] hover:text-red-500 rounded-md"
                                                    title="Remove"
                                                >
                                                    <Trash2 size={12} />
                                                </button>
                                            </div>
                                        </li>
                                    ))
                                ) : (
                                    <li className="px-3 py-4 text-center border-2 border-dashed border-[var(--border)] rounded-lg">
                                        <button
                                            onClick={addDirectory}
                                            className="text-xs text-[var(--muted)] hover:text-[var(--foreground)] flex flex-col items-center gap-1 w-full"
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
                            <h3 className="text-[11px] uppercase tracking-widest text-[var(--muted)] font-bold opacity-50">Collections</h3>
                            <button className="p-1 text-[var(--muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-3)] rounded transition-colors">
                                <Plus size={14} />
                            </button>
                        </div>
                        <ul className="space-y-1">
                            <li>
                                <button className="w-full flex items-center justify-between px-3 py-2 text-sm text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)] rounded-lg transition-colors">
                                    <span className="flex items-center gap-3">
                                        <span className="w-2.5 h-2.5 rounded-full bg-red-400"></span>
                                        Favorites
                                    </span>
                                    <span className="text-[10px] font-bold opacity-40">12</span>
                                </button>
                            </li>
                        </ul>
                    </div>

                    {/* Sources Section */}
                    <div>
                        <div className="flex items-center justify-between mb-3 px-2">
                            <h3 className="text-[11px] uppercase tracking-widest text-[var(--muted)] font-bold opacity-50">Sources</h3>
                        </div>
                        <ul className="space-y-1">
                            {sources.length === 0 ? (
                                <li className="px-3 py-2 text-[10px] text-[var(--muted)] opacity-50 text-center italic">
                                    Local system only
                                </li>
                            ) : (
                                sources.map(source => (
                                    <li key={source.id}>
                                        <button
                                            onClick={() => setSelectedSourceId(
                                                selectedSourceId === source.id ? null : source.id
                                            )}
                                            className={`w-full flex items-center justify-between px-3 py-2 text-sm rounded-lg transition-colors ${selectedSourceId === source.id
                                                ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
                                                : 'text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)]'
                                                }`}
                                        >
                                            <span className="flex items-center gap-3 truncate font-medium">
                                                {source.source_type === 'directory' ? (
                                                    <HardDrive size={15} className="flex-shrink-0 opacity-70" />
                                                ) : (
                                                    <Cloud size={15} className="flex-shrink-0 opacity-70" />
                                                )}
                                                <span className="truncate" title={source.name}>
                                                    {source.name}
                                                </span>
                                            </span>
                                            <span className="text-[10px] font-bold opacity-40 ml-2">
                                                {source.font_count || 0}
                                            </span>
                                        </button>
                                    </li>
                                ))
                            )}
                        </ul>
                    </div>


                </div>

                {/* Footer */}
                <div className="p-4 border-t border-[var(--border)] space-y-4">
                    {/* Qdrant Status */}
                    <button
                        onClick={() => setShowPreferences(true)}
                        className="w-full flex items-center justify-between px-3 py-2 text-xs text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--foreground)] rounded-lg transition-colors border border-transparent hover:border-[var(--border)]"
                    >
                        <span className="flex items-center gap-2 font-medium">
                            {settings.qdrantMode === 'local' ? <HardDrive size={14} /> : <Cloud size={14} />}
                            <span>AI Engine</span>
                        </span>
                        <div className="flex items-center gap-1.5">
                            <span className="text-[10px] opacity-40 uppercase font-bold">
                                {settings.qdrantMode === 'local' ? 'Local' : 'Remote'}
                            </span>
                            <span className="w-1.5 h-1.5 rounded-full bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]"></span>
                        </div>
                    </button>

                    {/* Action Buttons */}
                    <div className="flex items-center justify-between gap-1 px-1">
                        <button
                            onClick={() => setShowPreferences(true)}
                            className="p-2 text-[var(--muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-3)] rounded-lg transition-all"
                            title="Analytics"
                        >
                            <BarChart3 size={15} />
                        </button>
                        <button
                            onClick={() => setShowPreferences(true)}
                            className="p-2 text-[var(--muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-3)] rounded-lg transition-all"
                            title="Preferences"
                        >
                            <Settings size={15} />
                        </button>
                        <button
                            onClick={() => setShowCredits(true)}
                            className="p-2 text-[var(--muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-3)] rounded-lg transition-all"
                            title="About"
                        >
                            <Info size={15} />
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
