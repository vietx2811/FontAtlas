import React, { useState, useMemo, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
    Search, Map, Grid, RefreshCw,
    SlidersHorizontal, ChevronDown, ChevronUp, RotateCcw,
    Loader2, AlertCircle, Heart, Folder, CheckCircle2, XCircle, X
} from 'lucide-react';
import { Button } from './components/Button';
import { useStatus } from './context/StatusContext';
import { motion, AnimatePresence } from 'framer-motion';

// Components
import { Sidebar } from './components/Sidebar';
import { VirtualizedFontGrid } from './components/VirtualizedFontGrid';
import { DetailPanel } from './components/DetailPanel';
import { SimilarFontsPanel } from './components/SimilarFontsPanel';
import { PreviewPanel } from './components/PreviewPanel';

// Hooks & Types
import { useLibrary } from './hooks/useLibrary';
import { FontData, FontCategory } from './types';
import { FontMap, generateMockMapData, type FontMapNode } from './components/FontMap';
import { WindowShell } from './components/shell/WindowShell';

// --- Error Boundary ---
class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean }> {
    state = { hasError: false };
    static getDerivedStateFromError() { return { hasError: true }; }
    componentDidCatch(error: any, info: any) { console.error("Crash caught:", error, info); }
    render() {
        if (this.state.hasError) {
            return (
                <div className="min-h-screen flex items-center justify-center p-6 bg-[var(--background)]">
                    <div className="max-w-md w-full bg-[var(--card)] p-8 rounded-2xl border border-[var(--border)] shadow-xl">
                        <h2 className="text-xl font-bold text-[var(--foreground)] mb-2">Interface Error</h2>
                        <p className="text-[var(--muted)] text-sm mb-6">Something crashed in the UI. We've caught it to prevent data loss.</p>
                        <button onClick={() => window.location.reload()} className="w-full py-2 bg-[var(--foreground)] text-[var(--background)] rounded-lg font-bold hover:opacity-90 transition-all uppercase tracking-widest text-[11px]">
                            Restart App
                        </button>
                    </div>
                </div>
            );
        }
        return this.props.children;
    }
}

// --- Main App ---
const App: React.FC = () => {
    // Library Service
    const {
        fonts: libraryFonts,
        isLoading,
        error,
        fontCount,
        refetch,
        search: searchFonts,
        scanFolder,
        isDesktop,
        loadMore,
        filterByCategory
    } = useLibrary();

    // App State
    const [selectedCategory, setSelectedCategory] = useState<FontCategory | 'All'>('All');
    const [selectedPrimaryClass, setSelectedPrimaryClass] = useState<string | 'All'>('All');
    const [selectedTags, setSelectedTags] = useState<string[]>([]);
    const [searchQuery, setSearchQuery] = useState('');
    const [showFavoritesOnly, setShowFavoritesOnly] = useState(false);
    const [selectedDirectory, setSelectedDirectory] = useState<string | null>(null);
    const [selectedFont, setSelectedFont] = useState<FontData | null>(null);
    const [viewMode, setViewMode] = useState<'grid' | 'map'>('grid');
    const [gridSize, setGridSize] = useState(4); // 2-6 columns
    const { status: globalStatus, statusType, isBusy } = useStatus();
    const [similarPanelFont, setSimilarPanelFont] = useState<FontData | null>(null);
    const [previewFont, setPreviewFont] = useState<FontData | null>(null);

    // Debug: Log previewFont changes
    useEffect(() => {
        console.log('previewFont state changed:', previewFont?.name || 'null');
    }, [previewFont]);

    // Scanning State
    const [isScanning, setIsScanning] = useState(false);
    const [scanProgress, setScanProgress] = useState<{ count: number, total: number, font_name: string } | null>(null);
    const [scanResult, setScanResult] = useState<{ total: number, success: number, errors: number, error_details: string[] } | null>(null);
    const [showScanLogs, setShowScanLogs] = useState(false);

    // Advanced Filter State
    const [isFilterExpanded, setIsFilterExpanded] = useState(false);
    const [metricsFilter, setMetricsFilter] = useState({
        xHeight: [0, 100] as [number, number],
        contrast: [0, 100] as [number, number],
        width: [0, 200] as [number, number]
    });

    // Listen for backend updates
    useEffect(() => {
        const unlisteners: (() => void)[] = [];

        const setup = async () => {
            const up = await listen<any>('rescan-progress', (event) => {
                setScanProgress(event.payload);
                setIsScanning(true);
            });
            unlisteners.push(up);

            const uc = await listen<any>('rescan-complete', (event) => {
                setScanResult(event.payload);
                setIsScanning(false);
                setScanProgress(null);
                refetch();
                // Optionally show logs automatically if there are errors
                if (event.payload.errors > 0) setShowScanLogs(true);
            });
            unlisteners.push(uc);

            const ue = await listen<string>('rescan-error', (event) => {
                setScanResult({ total: 0, success: 0, errors: 1, error_details: [event.payload] });
                setIsScanning(false);
                setScanProgress(null);
                setShowScanLogs(true);
            });
            unlisteners.push(ue);
        };

        setup().catch(console.error);
        return () => unlisteners.forEach(u => u());
    }, [refetch]);

    // Transform Library -> UI Data
    const fonts: FontData[] = useMemo(() => {
        if (!libraryFonts) return [];
        return libraryFonts.map(f => ({
            id: f.id,
            name: f.name || 'Unknown Font',
            family: f.family_name || f.name || 'Unknown',
            category: (() => {
                const p = f.primary_class;
                if (p === 'serif') return FontCategory.Serif;
                if (p === 'sans') return FontCategory.SansSerif;
                if (p === 'script') return FontCategory.Script;
                if (p === 'monospace') return FontCategory.Monospace;
                return FontCategory.SansSerif;
            })(),
            filePath: f.file_path,
            metrics: {
                xHeightRatio: f.x_height || 0,
                widthVariance: f.width_variance || 0,
                strokeContrast: f.stroke_contrast || 0,
                aperture: f.aperture || 0,
                capHeight: 0,
                unitsPerEm: 1000,
                curvature: f.curvature_ratio || 0,
                terminalSharpness: f.terminal_sharpness || 0,
                stressAngle: f.stress_angle || 0,
                counterDensity: f.counter_density || 0
            },
            description: f.classification || '',
            tags: f.tags || [],
            fileSize: '',
            dateAnalyzed: f.date_added,
            primary_class: f.primary_class,
            secondary_tags: f.secondary_tags,
            isFavorite: f.is_favorite || false
        }));
    }, [libraryFonts]);

    // Derived available tags
    const availableTags = useMemo(() => {
        const tagSet = new Set<string>();
        libraryFonts?.forEach(f => {
            try {
                const t = typeof f.secondary_tags === 'string' ? JSON.parse(f.secondary_tags) : f.secondary_tags;
                if (Array.isArray(t)) t.forEach(tag => tagSet.add(tag));
            } catch { }
        });
        return Array.from(tagSet).sort();
    }, [libraryFonts]);

    const popularTags = useMemo(() =>
        availableTags.length > 0 ? availableTags : ['clean', 'modern', 'tech', 'elegant']
        , [availableTags]);

    // Filtering Logic
    const filteredFonts = useMemo(() => {
        return fonts.filter(f => {
            // 1. Text Search
            if (searchQuery) {
                const q = searchQuery.toLowerCase();
                if (!f.name.toLowerCase().includes(q) && !f.family.toLowerCase().includes(q)) return false;
            }
            // 2. Class
            if (selectedPrimaryClass !== 'All' && f.primary_class !== selectedPrimaryClass) return false;
            // 3. Tags
            if (selectedTags.length > 0) {
                try {
                    const raw = f.secondary_tags;
                    const ft = typeof raw === 'string' ? JSON.parse(raw || '[]') : raw;
                    if (!Array.isArray(ft) || !selectedTags.some(t => ft.includes(t))) return false;
                } catch { return false; }
            }
            // 4. Metrics - Skip if at max (no filter applied)
            // Only filter if user has adjusted the slider below max
            // (When at max, show all fonts)

            // 5. Favorites
            if (showFavoritesOnly && !f.isFavorite) return false;

            // 6. Directory/Folder
            if (selectedDirectory) {
                // Ensure we match fonts in this directory or subdirectories
                if (!f.filePath?.startsWith(selectedDirectory)) return false;
            }

            return true;
        });
    }, [fonts, searchQuery, selectedPrimaryClass, selectedTags, metricsFilter, showFavoritesOnly, selectedDirectory]);

    // Handlers
    const handleClear = () => {
        setSearchQuery('');
        setSelectedPrimaryClass('All');
        setSelectedTags([]);
        setShowFavoritesOnly(false);
        setSelectedDirectory(null);
        setMetricsFilter({ xHeight: [0, 0], contrast: [0, 0], width: [0, 0] });
    };

    return (
        <WindowShell sidebar={
            <Sidebar
                selectedCategory={selectedCategory}
                onSelectCategory={setSelectedCategory}
                fontCount={filteredFonts.length}
                onScanDirectory={scanFolder}
                showFavoritesOnly={showFavoritesOnly}
                onShowFavoritesChange={setShowFavoritesOnly}
                selectedDirectory={selectedDirectory}
                onSelectDirectory={setSelectedDirectory}
            />
        }>
            <main className="flex flex-col min-h-full relative text-[var(--foreground)]">
                {/* Fixed Header */}
                <header className="fixed top-0 right-0 left-[var(--sidebar-w)] z-30 px-6 py-3 bg-[var(--background)]/95 backdrop-blur-xl border-b border-[var(--border)]">
                    <div className="flex items-center gap-4">
                        {/* Search Bar */}
                        <div className="flex-1 flex items-center gap-2 px-4 py-2 bg-white border border-[var(--border)] rounded-full shadow-sm">
                            <Search size={16} className="text-[var(--muted)]" />

                            {/* Active Filters as Tags */}
                            {selectedPrimaryClass !== 'All' && (
                                <span
                                    className="inline-flex items-center gap-1 px-2 py-0.5 bg-stone-800 text-white text-[10px] font-bold uppercase rounded-full cursor-pointer"
                                    onClick={() => setSelectedPrimaryClass('All')}
                                >
                                    {selectedPrimaryClass} <X size={10} />
                                </span>
                            )}
                            {selectedTags.map(tag => (
                                <span
                                    key={tag}
                                    className="inline-flex items-center gap-1 px-2 py-0.5 bg-amber-600 text-white text-[10px] font-medium rounded-full cursor-pointer"
                                    onClick={() => setSelectedTags(selectedTags.filter(t => t !== tag))}
                                >
                                    {tag} <X size={10} />
                                </span>
                            ))}

                            <input
                                type="text"
                                placeholder="Search fonts..."
                                value={searchQuery}
                                onChange={e => setSearchQuery(e.target.value)}
                                className="flex-1 bg-transparent border-none text-sm placeholder:text-stone-400 focus:outline-none"
                            />
                        </div>

                        {/* Classification Filters */}
                        <div className="flex items-center gap-1">
                            {['All', 'Serif', 'Sans', 'Script', 'Display', 'Mono'].map(label => {
                                const id = label === 'All' ? 'All' : label === 'Sans' ? 'sans' : label === 'Mono' ? 'monospace' : label.toLowerCase();
                                const active = selectedPrimaryClass === id;
                                return (
                                    <button
                                        key={label}
                                        onClick={() => setSelectedPrimaryClass(id)}
                                        className={`px-2.5 py-1 text-[10px] uppercase font-bold tracking-wide rounded-full transition-all ${active ? 'bg-stone-800 text-white' : 'text-stone-500 hover:text-stone-800 hover:bg-stone-100'}`}
                                    >
                                        {label}
                                    </button>
                                );
                            })}
                        </div>

                        <div className="h-5 w-px bg-stone-200" />

                        {/* Grid Size */}
                        <div className="flex items-center gap-2">
                            <Grid size={14} className="text-stone-400" />
                            <input
                                type="range"
                                min="2" max="8"
                                value={gridSize}
                                onChange={(e) => setGridSize(Number(e.target.value))}
                                className="w-16 h-1 accent-stone-800"
                            />
                        </div>

                        {/* Filter Toggle */}
                        <button
                            onClick={() => setIsFilterExpanded(!isFilterExpanded)}
                            className={`p-2 rounded-full transition-all ${isFilterExpanded ? 'bg-stone-800 text-white' : 'bg-stone-100 text-stone-500 hover:bg-stone-200'}`}
                        >
                            <SlidersHorizontal size={14} />
                        </button>

                        {/* Sync Status */}
                        <AnimatePresence>
                            {(isScanning || scanResult) && (
                                <motion.button
                                    onClick={() => setShowScanLogs(true)}
                                    initial={{ opacity: 0, scale: 0.9 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    className={`flex items-center gap-2 px-3 py-1.5 rounded-full border text-[10px] font-bold uppercase tracking-wider ${isScanning ? 'bg-amber-50 border-amber-200 text-amber-600'
                                        : scanResult?.errors === 0 ? 'bg-green-50 border-green-200 text-green-600'
                                            : 'bg-red-50 border-red-200 text-red-500'
                                        }`}
                                >
                                    {isScanning ? (
                                        <>
                                            <Loader2 size={12} className="animate-spin" />
                                            <span>{Math.round((scanProgress?.count || 0) / (scanProgress?.total || 1) * 100)}%</span>
                                        </>
                                    ) : scanResult?.errors === 0 ? (
                                        <>
                                            <CheckCircle2 size={12} />
                                            <span>Synced</span>
                                        </>
                                    ) : (
                                        <>
                                            <XCircle size={12} />
                                            <span>{scanResult?.errors} Issues</span>
                                        </>
                                    )}
                                </motion.button>
                            )}
                        </AnimatePresence>

                        {/* Refresh */}
                        <button
                            onClick={refetch}
                            disabled={isLoading || isScanning}
                            className="p-2 bg-stone-100 rounded-full text-stone-500 hover:bg-stone-200 transition-all disabled:opacity-50"
                        >
                            <RefreshCw size={14} className={(isLoading || isScanning) ? 'animate-spin' : ''} />
                        </button>
                    </div>
                </header>

                {/* Expanded Filter Panel */}
                <AnimatePresence>
                    {isFilterExpanded && (
                        <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: 'auto', opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            className="fixed top-[60px] right-0 left-[var(--sidebar-w)] z-20 bg-white border-b border-stone-200 shadow-md px-6 py-4"
                        >
                            {/* Metric Sliders Row */}
                            <div className="flex items-center gap-8 mb-4 pb-4 border-b border-stone-100">
                                <MetricRange
                                    label="x-Height"
                                    min={0} max={100}
                                    value={metricsFilter.xHeight}
                                    onChange={v => setMetricsFilter(p => ({ ...p, xHeight: v }))}
                                />
                                <MetricRange
                                    label="Contrast"
                                    min={0} max={100}
                                    value={metricsFilter.contrast}
                                    onChange={v => setMetricsFilter(p => ({ ...p, contrast: v }))}
                                />
                                <MetricRange
                                    label="Width"
                                    min={0} max={200}
                                    value={metricsFilter.width}
                                    onChange={v => setMetricsFilter(p => ({ ...p, width: v }))}
                                />
                            </div>

                            {/* Attributes Section */}
                            <div className="mb-4">
                                <h3 className="text-[10px] uppercase tracking-widest font-bold text-stone-400 mb-3">Attributes</h3>
                                <div className="flex flex-wrap gap-2">
                                    {popularTags.map(tag => {
                                        const isActive = selectedTags.includes(tag);
                                        return (
                                            <button
                                                key={tag}
                                                onClick={() => {
                                                    setSelectedTags(isActive ? selectedTags.filter(t => t !== tag) : [...selectedTags, tag]);
                                                }}
                                                className={`px-3 py-1 text-[10px] rounded-full border transition-all capitalize ${isActive
                                                    ? 'bg-stone-800 text-white border-transparent font-bold'
                                                    : 'bg-stone-50 text-stone-500 border-stone-200 hover:border-stone-400'
                                                    }`}
                                            >
                                                {tag}
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>

                            {/* Clear Button */}
                            <div className="flex justify-end">
                                <button onClick={handleClear} className="flex items-center gap-1.5 text-[10px] uppercase font-bold tracking-widest text-stone-400 hover:text-red-500 transition-colors">
                                    <RotateCcw size={12} /> Clear All
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                {/* Scan Logs Modal */}
                <AnimatePresence>
                    {showScanLogs && scanResult && (
                        <div className="fixed inset-0 z-[100] flex items-center justify-center p-6">
                            <motion.div
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1 }}
                                exit={{ opacity: 0 }}
                                className="absolute inset-0 bg-stone-900/40 backdrop-blur-sm"
                                onClick={() => setShowScanLogs(false)}
                            />
                            <motion.div
                                initial={{ opacity: 0, scale: 0.95, y: 20 }}
                                animate={{ opacity: 1, scale: 1, y: 0 }}
                                exit={{ opacity: 0, scale: 0.95, y: 20 }}
                                className="relative w-full max-w-lg bg-white rounded-3xl shadow-2xl overflow-hidden border border-stone-200"
                            >
                                <div className="p-8">
                                    <div className="flex items-center justify-between mb-8">
                                        <h3 className="text-xl font-bold text-stone-900">Synchronization Log</h3>
                                        <button onClick={() => setShowScanLogs(false)} className="p-2 hover:bg-stone-100 rounded-full transition-colors"><X size={20} className="text-stone-400" /></button>
                                    </div>

                                    <div className="grid grid-cols-3 gap-4 mb-8">
                                        <div className="bg-stone-50 p-4 rounded-2xl text-center border border-stone-100">
                                            <div className="text-[10px] uppercase font-bold text-stone-400 mb-1">Total</div>
                                            <div className="text-2xl font-bold text-stone-900">{scanResult.total}</div>
                                        </div>
                                        <div className="bg-green-50 p-4 rounded-2xl text-center border border-green-100">
                                            <div className="text-[10px] uppercase font-bold text-green-600/60 mb-1">Success</div>
                                            <div className="text-2xl font-bold text-green-600">{scanResult.success}</div>
                                        </div>
                                        <div className="bg-red-50 p-4 rounded-2xl text-center border border-red-100">
                                            <div className="text-[10px] uppercase font-bold text-red-600/60 mb-1">Errors</div>
                                            <div className="text-2xl font-bold text-red-600">{scanResult.errors}</div>
                                        </div>
                                    </div>

                                    {scanResult.error_details.length > 0 ? (
                                        <div className="space-y-3 max-h-64 overflow-y-auto custom-scroll pr-2">
                                            <h4 className="text-[10px] uppercase font-bold text-stone-400 tracking-widest">Detail Logs</h4>
                                            {scanResult.error_details.map((err, i) => (
                                                <div key={i} className="p-3 bg-red-50/50 border border-red-100 rounded-xl text-xs text-red-700 leading-relaxed">
                                                    {err}
                                                </div>
                                            ))}
                                        </div>
                                    ) : (
                                        <div className="py-12 text-center">
                                            <CheckCircle2 size={40} className="mx-auto text-green-500 mb-4 opacity-20" />
                                            <p className="text-stone-400 text-sm">Everything synchronized perfectly.</p>
                                        </div>
                                    )}

                                    <button
                                        onClick={() => setShowScanLogs(false)}
                                        className="w-full mt-8 py-4 bg-stone-900 text-white rounded-2xl font-bold hover:bg-stone-800 transition-all shadow-lg active:scale-[0.98]"
                                    >
                                        Dismiss Log
                                    </button>
                                </div>
                            </motion.div>
                        </div>
                    )
                    }
                </AnimatePresence >

                <div className="flex-1 p-6 pt-20 relative">
                    {isLoading ? (
                        <div className="h-full flex flex-col items-center justify-center space-y-4 opacity-50">
                            <Loader2 className="w-8 h-8 animate-spin" />
                            <p className="text-xs uppercase tracking-[0.2em] font-medium">Synchronizing Library...</p>
                        </div>
                    ) : filteredFonts.length === 0 ? (
                        <div className="h-full flex flex-col items-center justify-center space-y-4">
                            <div className="w-16 h-16 rounded-full bg-[var(--surface-2)] flex items-center justify-center border border-[var(--border)]">
                                <Search className="w-6 h-6 text-[var(--muted)]" />
                            </div>
                            <div className="text-center">
                                <h3 className="text-sm font-semibold text-[var(--foreground)]">No Matching Typefaces</h3>
                                <p className="text-xs text-[var(--muted)] mt-1">Try adjusting your filters or search query</p>
                            </div>
                            <Button variant="secondary" onClick={handleClear} className="mt-2 text-[10px] py-1 px-4 uppercase tracking-widest font-bold">
                                Reset Filters
                            </Button>
                        </div>
                    ) : viewMode === 'grid' ? (
                        <VirtualizedFontGrid
                            fonts={filteredFonts}
                            selectedFontId={selectedFont?.id}
                            onFontSelect={(font) => {
                                setSelectedFont(font);
                                setPreviewFont(null); // Reset preview when selecting new font
                            }}
                            onFindSimilar={setSimilarPanelFont}
                            isLoading={isLoading}
                            onLoadMore={loadMore}
                            hasMore={filteredFonts.length < fontCount}
                            columnCount={gridSize}
                        />
                    ) : (
                        <div className="h-full bg-[var(--surface-2)] rounded-3xl overflow-hidden border border-[var(--border)]">
                            <FontMap
                                fonts={generateMockMapData(fonts)}
                                width={window.innerWidth - 300}
                                height={window.innerHeight - 200}
                                onFontSelect={f => {
                                    const font = fonts.find(x => x.id === f.id) || null;
                                    setSelectedFont(font);
                                    setPreviewFont(null); // Reset preview when selecting new font
                                }}
                                selectedFontId={selectedFont?.id}
                            />
                        </div>
                    )}
                </div>
            </main >

            {/* Overlays */}
            {/* Backdrop */}
            {
                selectedFont && (
                    <div
                        className="fixed inset-0 bg-black/20 z-40"
                        onClick={() => {
                            setSelectedFont(null);
                            setPreviewFont(null);
                        }}
                    />
                )
            }

            <AnimatePresence>
                {selectedFont && (
                    <DetailPanel
                        font={selectedFont}
                        onClose={() => setSelectedFont(null)}
                        library={fonts}
                        onSelectFont={setSelectedFont}
                        onPreview={(font) => {
                            console.log('Preview triggered from DetailPanel:', font.name);
                            setPreviewFont(font);
                        }}
                        hasPreviewPanel={!!previewFont}
                    />
                )}
                {similarPanelFont && (
                    <SimilarFontsPanel
                        font={similarPanelFont}
                        onClose={() => setSimilarPanelFont(null)}
                        onFontSelect={id => {
                            const f = fonts.find(x => x.id === id);
                            if (f) { setSelectedFont(f); setSimilarPanelFont(null); }
                        }}
                    />
                )}
                {previewFont && (
                    <PreviewPanel
                        key={previewFont.id}
                        font={previewFont}
                        onClose={() => setPreviewFont(null)}
                        onGoToFont={(font) => {
                            setSelectedFont(font);
                            setPreviewFont(null);
                        }}
                    />
                )}
            </AnimatePresence>
        </WindowShell>
    );
};

// Simple Metric Range Slider
interface MetricRangeProps {
    label: string;
    min: number;
    max: number;
    value: [number, number];
    onChange: (v: [number, number]) => void;
}

const MetricRange: React.FC<MetricRangeProps> = ({ label, min, max, value, onChange }) => (
    <div className="flex items-center gap-2">
        <span className="text-[11px] font-bold uppercase tracking-wider text-stone-500 w-20">{label}</span>
        <span className="text-[11px] text-stone-700 w-6 text-right">{value[0]}</span>
        <input
            type="range"
            min={min}
            max={max}
            value={value[1]}
            onChange={e => onChange([value[0], parseInt(e.target.value)])}
            className="w-24 h-1 bg-stone-200 rounded-full appearance-none cursor-pointer accent-stone-800"
        />
        <span className="text-[11px] text-stone-700 w-6">{value[1]}</span>
    </div>
);

export default function AppWrapper() {
    return <ErrorBoundary><App /></ErrorBoundary>;
}
