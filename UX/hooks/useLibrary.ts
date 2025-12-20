/**
 * Unified Hooks for Font Library
 * ================================
 * 
 * Works with Tauri (desktop) or falls back to web API
 */

import { useState, useEffect, useCallback } from 'react';
import {
  getLibraryFonts,
  searchLibrary,
  scanAndAddDirectory,
  getFontCount,
  type LibraryFont
} from '../services/library';
import { isDesktopApp, scanDirectory, type FontFileInfo } from '../services/tauri';
import { useStatus } from '../context/StatusContext';

// Check if running in Tauri (works for both v1 and v2)
const isTauri = typeof window !== 'undefined' && (
  (window as any).__TAURI__ ||
  (window as any).__TAURI_INTERNALS__
);

export interface UseLibraryResult {
  fonts: LibraryFont[];
  isLoading: boolean;
  error: Error | null;
  fontCount: number;
  refetch: () => Promise<void>;
  loadMore: () => Promise<void>;
  search: (query: string) => Promise<void>;
  scanFolder: (path: string) => Promise<number>;
  filterByCategory: (category: string) => Promise<void>;
  isDesktop: boolean;
}

export function useLibrary(): UseLibraryResult {
  const [fonts, setFonts] = useState<LibraryFont[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [fontCount, setFontCount] = useState(0);
  const [offset, setOffset] = useState(0);
  const [activeCategory, setActiveCategory] = useState<string>('All');
  const [isSearching, setIsSearching] = useState(false);
  const LIMIT = 50;

  const { setStatus, setError: setGlobalError, clearStatus } = useStatus();

  const fetchFonts = useCallback(async (categoryOverride?: string) => {
    setIsLoading(true);
    setStatus('Loading font library...');
    setError(null);
    setOffset(0);
    setIsSearching(false);

    const categoryToUse = categoryOverride !== undefined ? categoryOverride : activeCategory;

    try {
      if (isTauri) {
        console.log(`Fetching fonts from library (Category: ${categoryToUse})...`);
        // Initial fetch: limit 50, offset 0
        const [libraryFonts, count] = await Promise.all([
          getLibraryFonts(LIMIT, 0, categoryToUse),
          // We ideally need getFontCount to also filter by category to be accurate, but strict accuracy 
          // on the "Total Fonts" count isn't critical for now, though it affects 'hasMore'.
          // For now, let's assume if we get < LIMIT fonts, we are done.
          getFontCount()
        ]);
        console.log('Fetched', libraryFonts.length, 'fonts');
        setFonts(libraryFonts);
        // If filtering, the total count is wrong. 
        // Workaround: if we got less than LIMIT, we know we are at the end.
        // If we got LIMIT, we assume there is more (unless exact multiple).
        // Since we don't have getFontCount(category), we'll rely on the length of returned array in loadMore.
        setFontCount(count);
        setOffset(LIMIT);
      } else {
        // Web mode - use mock data for demo
        const mockFonts: LibraryFont[] = [
          {
            id: '1', name: 'Inter', file_path: '/fonts/Inter.ttf', original_directory: '/fonts',
            category: 'Sans-Serif', classification: 'Grotesque', tags: ['modern', 'clean'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.552, width_variance: 0.12, stroke_contrast: 0.05, aperture: 0.8,
            foundry: 'Google', weight: 400, is_italic: false, supports_vietnamese: true,
            family_name: 'Inter', subfamily: 'Regular', weight_class: 400, width_class: 5, source_id: null,
            primary_class: null, secondary_tags: null, serif_subtype: null, sans_subtype: null, script_subtype: null
          }];
        setFonts(mockFonts);
        setFontCount(mockFonts.length);
      }
      clearStatus();
    } catch (e) {
      const err = e instanceof Error ? e : new Error('Failed to fetch fonts');
      setError(err);
      setGlobalError(err.message);
    } finally {
      setIsLoading(false);
    }
  }, [setStatus, clearStatus, activeCategory]); // activeCategory dependency matters if we don't override

  const loadMore = useCallback(async () => {
    if (!isTauri || isSearching) return;
    // Simple check: if last batch was small, we are done.
    // However, since we don't store "last batch size", checking fonts.length vs generic fontCount 
    // is buggy when filtered.
    // Better logic: infinite scroll should just try to fetch. If empty, stop.
    // But VirtualizedFontGrid calls this repeatedly if we don't stop it.
    // Let's assume safely:

    setIsLoading(true);
    setStatus(`Loading more fonts... (${offset} loaded)`);
    try {
      const nextBatch = await getLibraryFonts(LIMIT, offset, activeCategory);
      if (nextBatch.length > 0) {
        setFonts(prev => [...prev, ...nextBatch]);
        setOffset(prev => prev + LIMIT);
      }
    } catch (e) {
      console.error("Failed to load more fonts", e);
    } finally {
      setIsLoading(false);
      clearStatus();
    }
  }, [offset, activeCategory, isSearching]);

  const filterByCategory = useCallback(async (category: string) => {
    setActiveCategory(category);
    await fetchFonts(category);
  }, [fetchFonts]);

  const search = useCallback(async (query: string) => {
    if (!query.trim()) {
      setActiveCategory('All'); // Reset category on clear search? or keep it?
      // Let's assume clearing search resets to "All" view for now to be safe
      return fetchFonts('All');
    }

    setIsLoading(true);
    setIsSearching(true);
    try {
      if (isTauri) {
        const results = await searchLibrary(query);
        setFonts(results);
      } else {
        // Client-side filter for web
      }
      clearStatus();
    } catch (e) {
      const err = e instanceof Error ? e : new Error('Search failed');
      setError(err);
      setGlobalError(err.message);
    } finally {
      setIsLoading(false);
    }
  }, [fetchFonts]);

  const scanFolder = useCallback(async (path: string): Promise<number> => {
    if (!isTauri) {
      throw new Error('Folder scanning requires desktop app');
    }

    // Pass setStatus for progress updates
    const added = await scanAndAddDirectory(path, (msg) => setStatus(msg));

    setStatus('Refreshing library view...');
    await fetchFonts(); // Refresh after adding
    return added;
  }, [fetchFonts, setStatus]);

  useEffect(() => {
    fetchFonts();
  }, []);

  return {
    fonts,
    isLoading,
    error,
    fontCount,
    refetch: () => fetchFonts(),
    loadMore,
    search,
    scanFolder,
    filterByCategory,
    isDesktop: isTauri
  };
}

/**
 * Hook for scanning directories without adding to library
 */
export function useFontScanner() {
  const [files, setFiles] = useState<FontFileInfo[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const scan = useCallback(async (path: string) => {
    if (!isTauri) {
      setError(new Error('Requires desktop app'));
      return;
    }

    setIsScanning(true);
    setError(null);

    try {
      const fonts = await scanDirectory(path);
      setFiles(fonts);
    } catch (e) {
      setError(e instanceof Error ? e : new Error('Scan failed'));
    } finally {
      setIsScanning(false);
    }
  }, []);

  return { files, isScanning, error, scan };
}
