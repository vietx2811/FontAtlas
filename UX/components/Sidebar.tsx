import React, { useState } from 'react';
import { LayoutGrid, Settings, FolderOpen, Database, BarChart3, Info, Plus, Folder, ChevronRight, Cloud, HardDrive } from 'lucide-react';
import { FontCategory } from '../types';
import { CollectionsManager, type Collection } from './CollectionsManager';
import { CreditsPage } from './CreditsPage';
import { PreferencesPanel } from './PreferencesPanel';
// import { DirectoriesPanel } from './DirectoriesPanel'; // Deprecated
import { useSettings } from '../hooks/useSettings';
import { useDirectories } from '../hooks/useDirectories';
import { clearLibrary } from '../services/library';
import { Loader2, Trash2, RefreshCw } from 'lucide-react';

interface SidebarProps {
    selectedCategory: FontCategory | 'All';
    onSelectCategory: (cat: FontCategory | 'All') => void;
    fontCount: number;
    onSelectCollection?: (collection: Collection | null) => void;
    selectedCollectionId?: string | null;
    onScanDirectory?: (path: string) => Promise<number>;
}

export const Sidebar: React.FC<SidebarProps> = ({
    selectedCategory,
    onSelectCategory,
    fontCount,
    onSelectCollection,
    selectedCollectionId,
    onScanDirectory
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

    return (
        <>
            <aside className="w-64 border-r border-stone-200 bg-white h-screen fixed left-0 top-0 flex flex-col z-30 hidden md:flex">
                {/* Logo Header - matches main header: px-6 py-3 */}
                <div className="flex items-center px-6 py-3 shrink-0">
                    <div className="w-9 h-9 bg-gradient-to-br from-amber-400 to-orange-500 rounded-lg mr-3 flex items-center justify-center">
                        <span className="text-white text-sm font-bold">F</span>
                    </div>
                    <span className="font-semibold text-lg text-stone-900 tracking-tight">FontAtlas</span>
                </div>

                {/* Main Nav - starts aligned with main content grid (after header border) */}
                <div className="flex-1 overflow-y-auto pt-6 pb-4 px-4 space-y-6">

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

                    {/* Tags Section */}
                    <div>
                        <div className="flex items-center justify-between mb-3 px-2">
                            <h3 className="text-[11px] uppercase tracking-widest text-stone-400 font-semibold">Tags</h3>
                            <ChevronRight size={14} className="text-stone-300" />
                        </div>
                        <div className="flex flex-wrap gap-1.5 px-2">
                            {['Modern', 'Classic', 'Elegant', 'UI', 'Editorial'].map(tag => (
                                <span key={tag} className="px-2 py-0.5 text-xs bg-stone-100 text-stone-500 rounded-full hover:bg-stone-200 cursor-pointer transition-colors">
                                    {tag}
                                </span>
                            ))}
                        </div>
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
