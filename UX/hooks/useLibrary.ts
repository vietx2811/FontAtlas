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
  search: (query: string) => Promise<void>;
  scanFolder: (path: string) => Promise<number>;
  isDesktop: boolean;
}

export function useLibrary(): UseLibraryResult {
  const [fonts, setFonts] = useState<LibraryFont[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [fontCount, setFontCount] = useState(0);
  const { setStatus, setError: setGlobalError, clearStatus } = useStatus();

  const fetchFonts = useCallback(async () => {
    setIsLoading(true);
    setStatus('Loading font library...');
    setError(null);

    try {
      if (isTauri) {
        console.log('Fetching fonts from library...');
        const [libraryFonts, count] = await Promise.all([
          getLibraryFonts(),
          getFontCount()
        ]);
        console.log('Fetched', libraryFonts.length, 'fonts, count:', count);
        setFonts(libraryFonts);
        setFontCount(count);
      } else {
        // Web mode - use mock data for demo
        // In a real scenario, this would connect to a backend API
        const mockFonts: LibraryFont[] = [
          {
            id: '1', name: 'Inter', file_path: '/fonts/Inter.ttf', original_directory: '/fonts',
            category: 'Sans-Serif', classification: 'Grotesque', tags: ['modern', 'clean'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.552, width_variance: 0.12, stroke_contrast: 0.05, aperture: 0.8
          },
          {
            id: '2', name: 'Libre Baskerville', file_path: '/fonts/LibreBaskerville.ttf', original_directory: '/fonts',
            category: 'Serif', classification: 'Transitional', tags: ['classic', 'editorial'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.48, width_variance: 0.18, stroke_contrast: 0.45, aperture: 0.6
          },
          {
            id: '3', name: 'Playfair Display', file_path: '/fonts/PlayfairDisplay.ttf', original_directory: '/fonts',
            category: 'Serif', classification: 'Modern', tags: ['elegant', 'display'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.46, width_variance: 0.22, stroke_contrast: 0.65, aperture: 0.55
          },
          {
            id: '4', name: 'Roboto', file_path: '/fonts/Roboto.ttf', original_directory: '/fonts',
            category: 'Sans-Serif', classification: 'Neo-Grotesque', tags: ['google', 'material'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.53, width_variance: 0.1, stroke_contrast: 0.08, aperture: 0.75
          },
          {
            id: '5', name: 'Montserrat', file_path: '/fonts/Montserrat.ttf', original_directory: '/fonts',
            category: 'Sans-Serif', classification: 'Geometric', tags: ['modern', 'geometric'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.54, width_variance: 0.08, stroke_contrast: 0.03, aperture: 0.82
          },
          {
            id: '6', name: 'Lora', file_path: '/fonts/Lora.ttf', original_directory: '/fonts',
            category: 'Serif', classification: 'Transitional', tags: ['reading', 'book'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.49, width_variance: 0.15, stroke_contrast: 0.35, aperture: 0.62
          },
          {
            id: '7', name: 'Source Code Pro', file_path: '/fonts/SourceCodePro.ttf', original_directory: '/fonts',
            category: 'Monospace', classification: 'Monospace', tags: ['code', 'developer'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.52, width_variance: 0.0, stroke_contrast: 0.1, aperture: 0.7
          },
          {
            id: '8', name: 'Dancing Script', file_path: '/fonts/DancingScript.ttf', original_directory: '/fonts',
            category: 'Script', classification: 'Casual Script', tags: ['handwriting', 'casual'],
            collection_id: null, date_added: new Date().toISOString(), date_modified: new Date().toISOString(),
            is_synced: false, file_exists: true, x_height: 0.45, width_variance: 0.35, stroke_contrast: 0.55, aperture: 0.5
          },
        ];
        setFonts(mockFonts);
        setFontCount(mockFonts.length);
      }
    } catch (e) {
      const err = e instanceof Error ? e : new Error('Failed to fetch fonts');
      setError(err);
      setGlobalError(err.message);
    } finally {
      setIsLoading(false);
      // clearStatus(); // Handled by setStatus/setError
    }
  }, [setStatus, clearStatus]);

  const search = useCallback(async (query: string) => {
    if (!query.trim()) {
      return fetchFonts();
    }

    setIsLoading(true);
    try {
      if (isTauri) {
        const results = await searchLibrary(query);
        setFonts(results);
      } else {
        // Client-side filter for web
        const all = await getLibraryFonts();
        const filtered = all.filter(f =>
          f.name.toLowerCase().includes(query.toLowerCase())
        );
        setFonts(filtered);
      }
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

    const added = await scanAndAddDirectory(path);
    await fetchFonts(); // Refresh after adding
    return added;
  }, [fetchFonts]);

  useEffect(() => {
    fetchFonts();
  }, [fetchFonts]);

  return {
    fonts,
    isLoading,
    error,
    fontCount,
    refetch: fetchFonts,
    search,
    scanFolder,
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
