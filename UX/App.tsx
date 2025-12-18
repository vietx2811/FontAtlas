import React, { useState, useMemo, useCallback } from 'react';
import { Sidebar } from './components/Sidebar';
import { FontCard } from './components/FontCard';
import { VirtualizedFontGrid } from './components/VirtualizedFontGrid';
import { DetailPanel } from './components/DetailPanel';
import { StatusBar } from './components/StatusBar';
import { Button } from './components/Button';
import { FontData, FontCategory } from './types';
import { Search, Map, Grid, RefreshCw } from 'lucide-react';
import { useLibrary } from './hooks/useLibrary';
import { FontMap, generateMockMapData, type FontMapNode } from './components/FontMap';
import { MOCK_FONTS } from './constants';

type ViewMode = 'grid' | 'map';

const App: React.FC = () => {
    // Library data hook (works with Tauri or web fallback)
    const {
        fonts: libraryFonts,
        isLoading,
        error,
        fontCount,
        refetch,
        search: searchFonts,
        scanFolder,
        isDesktop
    } = useLibrary();

    // Transform library fonts to UI FontData format OR use mock fonts
    const fonts: FontData[] = useMemo(() => {
        if (libraryFonts.length > 0) {
            return libraryFonts.map(font => ({
                id: font.id,
                name: font.name,
                family: font.name,
                category: (font.category || FontCategory.SansSerif) as FontCategory,
                filePath: font.file_path,
                metrics: {
                    xHeightRatio: font.x_height || 0,
                    widthVariance: font.width_variance || 0,
                    strokeContrast: font.stroke_contrast || 0,
                    aperture: font.aperture || 0,
                    capHeight: 0,
                    unitsPerEm: 1000
                },
                description: font.classification || '',
                tags: font.tags || [],
                fileSize: '',
                dateAnalyzed: font.date_added
            }));
        }
        // Return empty if no fonts found
        return [];
    }, [libraryFonts]);

    // UI State
    const [selectedCategory, setSelectedCategory] = useState<FontCategory | 'All'>('All');
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedFont, setSelectedFont] = useState<FontData | null>(null);
    const [viewMode, setViewMode] = useState<ViewMode>('grid');
    const [isScanning, setIsScanning] = useState(false);

    // Filter Logic
    const filteredFonts = useMemo(() => {
        return fonts.filter(font => {
            const matchesCategory = selectedCategory === 'All' || font.category === selectedCategory;
            const matchesSearch = font.name.toLowerCase().includes(searchQuery.toLowerCase());
            return matchesCategory && matchesSearch;
        });
    }, [selectedCategory, searchQuery, fonts]);

    // Map data
    const mapNodes: FontMapNode[] = useMemo(() => {
        return generateMockMapData(
            filteredFonts.map(f => ({
                id: f.id,
                name: f.name,
                category: f.category,
                filePath: f.filePath
            }))
        );
    }, [filteredFonts]);

    // Handlers
    const handleSearch = useCallback(async (e: React.FormEvent) => {
        e.preventDefault();
        if (isDesktop) {
            await searchFonts(searchQuery);
        }
    }, [searchQuery, searchFonts, isDesktop]);

    // Folder scanning logic moved to Sidebar/useDirectories hook

    const handleFontSelect = useCallback((font: FontData) => {
        setSelectedFont(font);
    }, []);

    const handleMapFontSelect = useCallback((node: FontMapNode) => {
        const font = fonts.find(f => f.id === node.id);
        if (font) setSelectedFont(font);
    }, [fonts]);

    return (
        <div className="min-h-screen bg-stone-50 text-stone-900">
            {/* Sidebar - fixed position */}
            <Sidebar
                selectedCategory={selectedCategory}
                onSelectCategory={setSelectedCategory}
                fontCount={fonts.length}
                onScanDirectory={scanFolder}
            />

            {/* Main Content - offset by sidebar width (w-64 = 16rem = 256px) */}
            <main className="md:ml-64 min-h-screen flex flex-col">
                {/* Sticky Header/Action Bar */}
                <header className="sticky top-0 z-20 flex items-center gap-4 px-6 py-3 border-b border-stone-200 bg-white/95 backdrop-blur-sm">
                    {/* Search */}
                    <div className="flex-1 max-w-md">
                        <form onSubmit={handleSearch} className="relative">
                            <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-stone-400" />
                            <input
                                type="text"
                                placeholder="Search fonts..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                className="w-full pl-9 pr-4 py-2 text-sm bg-stone-100 border border-stone-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-stone-900/20 focus:border-stone-400"
                            />
                        </form>
                    </div>

                    {/* Filter Tabs - like mockup */}
                    <div className="flex items-center gap-1 bg-stone-100 rounded-lg p-1">
                        {['All', 'Serif', 'Sans', 'Mono', 'Display'].map((filter) => {
                            const getCategory = (f: string) => {
                                if (f === 'All') return 'All';
                                if (f === 'Sans') return FontCategory.SansSerif;
                                if (f === 'Mono') return FontCategory.Monospace;
                                return f as FontCategory;
                            };
                            const targetCategory = getCategory(filter);

                            return (
                                <button
                                    key={filter}
                                    onClick={() => setSelectedCategory(targetCategory)}
                                    className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${selectedCategory === targetCategory
                                        ? 'bg-white shadow-sm text-stone-900'
                                        : 'text-stone-500 hover:text-stone-700'
                                        }`}
                                >
                                    {filter}
                                </button>
                            );
                        })}
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-2 ml-4">
                        {/* View toggle */}
                        <div className="flex bg-stone-100 rounded-lg p-0.5">
                            <button
                                className={`p-1.5 rounded-md transition-colors ${viewMode === 'grid' ? 'bg-white shadow-sm text-stone-900' : 'text-stone-400 hover:text-stone-600'}`}
                                onClick={() => setViewMode('grid')}
                                title="Grid View"
                            >
                                <Grid size={16} />
                            </button>
                            <button
                                className={`p-1.5 rounded-md transition-colors ${viewMode === 'map' ? 'bg-white shadow-sm text-stone-900' : 'text-stone-400 hover:text-stone-600'}`}
                                onClick={() => setViewMode('map')}
                                title="Map View"
                            >
                                <Map size={16} />
                            </button>
                        </div>

                        {/* Folder add moved to sidebar */}

                        <button
                            onClick={refetch}
                            disabled={isLoading}
                            className="p-1.5 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-lg transition-colors"
                        >
                            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
                        </button>
                    </div>
                </header>

                {/* Error Display */}
                {error && (
                    <div className="px-6 py-2 bg-red-50 border-b border-red-200 text-red-700 text-sm">
                        ⚠️ {error.message}
                    </div>
                )}

                {/* Content Area */}
                <div className="flex-1 overflow-hidden p-6">
                    {viewMode === 'grid' ? (
                        /* Grid View - Virtualized for performance */
                        <VirtualizedFontGrid
                            fonts={filteredFonts}
                            selectedFontId={selectedFont?.id}
                            onFontSelect={handleFontSelect}
                            isLoading={isLoading}
                        />
                    ) : (
                        /* Map View */
                        <div className="flex justify-center items-center h-full min-h-[500px] bg-stone-100 rounded-xl overflow-hidden">
                            <FontMap
                                fonts={mapNodes}
                                width={Math.min(1000, window.innerWidth - 400)}
                                height={Math.min(600, window.innerHeight - 250)}
                                onFontSelect={handleMapFontSelect}
                                selectedFontId={selectedFont?.id}
                            />
                        </div>
                    )}
                </div>
            </main>

            {/* Detail Panel */}
            {selectedFont && (
                <DetailPanel
                    font={selectedFont}
                    onClose={() => setSelectedFont(null)}
                    library={fonts}
                />
            )}
            {/* Status Bar Overlay */}
            <StatusBar />
        </div>
    );
};

export default App;
