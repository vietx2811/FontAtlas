/**
 * Directory Management Hook
 * ==========================
 * 
 * Manages the list of watched font directories containing:
 * - List of directories (persisted to localStorage)
 * - Add/Remove actions
 * - Scan/Rescan logic
 */

import { useState, useEffect, useCallback } from 'react';
import { useStatus } from '../context/StatusContext';

const STORAGE_KEY = 'fontatlas_directories';

import { ask } from '@tauri-apps/plugin-dialog';

export interface WatchedDirectory {
    path: string;
    fontCount: number;
    lastScanned: string;
}

export interface UseDirectoriesResult {
    directories: WatchedDirectory[];
    scanningPath: string | null;
    scanResult: { path: string; count: number; success: boolean } | null;
    isClearing: boolean;
    addDirectory: () => Promise<void>;
    rescanDirectory: (dir: WatchedDirectory) => Promise<void>;
    removeDirectory: (path: string) => void;
    clearAll: () => Promise<void>;
    scanPath: (path: string) => Promise<void>;
}

export function useDirectories(
    onScanDirectory?: (path: string) => Promise<number>,
    clearLibraryCallback?: () => Promise<number>
): UseDirectoriesResult {
    const [directories, setDirectories] = useState<WatchedDirectory[]>([]);
    const [scanningPath, setScanningPath] = useState<string | null>(null);
    const [scanResult, setScanResult] = useState<{ path: string; count: number; success: boolean } | null>(null);
    const [isClearing, setIsClearing] = useState(false);
    const { setStatus, setError, clearStatus } = useStatus();

    // Load from localStorage on mount
    useEffect(() => {
        try {
            const stored = localStorage.getItem(STORAGE_KEY);
            if (stored) {
                setDirectories(JSON.parse(stored));
            }
        } catch (e) {
            console.error('Failed to load directories:', e);
        }
    }, []);

    // Helper to save to localStorage
    const saveDirectories = useCallback((dirs: WatchedDirectory[]) => {
        setDirectories(dirs);
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(dirs));
        } catch (e) {
            console.error('Failed to save directories:', e);
        }
    }, []);

    // Core scan logic
    const scanPath = useCallback(async (path: string) => {
        if (!path || !path.trim() || !onScanDirectory) return;

        // Check if already exists (unless rescanning)
        const isExisting = directories.some(d => d.path === path);

        setScanningPath(path);
        setScanResult(null);

        try {
            setStatus('Scanning directory...');
            const count = await onScanDirectory(path);
            setStatus(`Added ${count} fonts`, false);
            setTimeout(clearStatus, 3000);
            const newDir: WatchedDirectory = {
                path,
                fontCount: count,
                lastScanned: new Date().toISOString()
            };

            if (isExisting) {
                // Update existing
                const updated = directories.map(d => d.path === path ? newDir : d);
                saveDirectories(updated);
            } else {
                // Add new
                saveDirectories([...directories, newDir]);
            }

            setScanResult({ path, count, success: true });
        } catch (e) {
            console.error('Scan failed:', e);
            setError(e instanceof Error ? e.message : 'Failed to scan directory');
            setScanResult({ path, count: 0, success: false });
        } finally {
            setScanningPath(null);
        }
    }, [directories, onScanDirectory, saveDirectories, setStatus, clearStatus, setError]);

    // UI Action: Add Directory (via dialog)
    const addDirectory = useCallback(async () => {
        try {
            const { invoke } = await import('@tauri-apps/api/core');
            const path = await invoke<string | null>('open_folder_dialog');

            if (path) {
                await scanPath(path);
            }
        } catch (e) {
            console.error('Failed to open folder dialog:', e);
        }
    }, [scanPath]);

    // UI Action: Rescan
    const rescanDirectory = useCallback(async (dir: WatchedDirectory) => {
        await scanPath(dir.path);
    }, [scanPath]);

    // UI Action: Remove
    const removeDirectory = useCallback(async (path: string) => {
        const yes = await ask('Remove this directory from your library? (Fonts won\'t be deleted)', {
            title: 'Confirm Removal',
            kind: 'warning'
        });

        if (yes) {
            saveDirectories(directories.filter(d => d.path !== path));
        }
    }, [directories, saveDirectories]);

    // UI Action: Clear All
    const clearAll = useCallback(async () => {
        if (!clearLibraryCallback) return;

        const yes = await ask('Clear all fonts from library? This will remove all scanned fonts.', {
            title: 'Clear Library',
            kind: 'warning'
        });

        if (!yes) {
            return;
        }

        setIsClearing(true);
        setStatus('Clearing library...');
        try {
            const deleted = await clearLibraryCallback();
            saveDirectories([]);
            setScanResult({ path: 'Library', count: deleted, success: true });
            setStatus(`Cleared available fonts`, false);
            setTimeout(clearStatus, 3000);
        } catch (e) {
            console.error('Failed to clear library:', e);
            setError(e instanceof Error ? e.message : 'Failed to clear library');
        } finally {
            setIsClearing(false);
            // Error cleared by timeout, Success cleared by timeout
        }
    }, [clearLibraryCallback, saveDirectories, setStatus, setError]);

    return {
        directories,
        scanningPath,
        scanResult,
        isClearing,
        addDirectory,
        rescanDirectory,
        removeDirectory,
        clearAll,
        scanPath
    };
}
