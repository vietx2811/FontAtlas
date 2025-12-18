/**
 * Tauri Service Layer
 * ====================
 * 
 * Bridges React frontend to Tauri Rust backend.
 * Uses Tauri's invoke() for IPC with native code.
 */

// Import Tauri API (only available in desktop app)
const isTauri = typeof window !== 'undefined' && (window as any).__TAURI__;

interface TauriInvoke {
    (cmd: string, args?: Record<string, unknown>): Promise<any>;
}

// Get invoke function if available
const getInvoke = (): TauriInvoke | null => {
    if (isTauri) {
        return (window as any).__TAURI__.core.invoke;
    }
    return null;
};

// Font file info from scan
export interface FontFileInfo {
    path: string;
    name: string;
    size: number;
}

/**
 * Scan a directory for font files
 */
export async function scanDirectory(path: string): Promise<FontFileInfo[]> {
    const invoke = getInvoke();
    if (!invoke) {
        console.warn('Tauri not available, using fallback API');
        // Fallback to web API
        const response = await fetch(`http://localhost:8000/api/fonts/scan?directory=${encodeURIComponent(path)}`);
        const data = await response.json();
        return data.files || [];
    }

    return invoke('scan_directory', { path });
}

/**
 * Read font file as bytes (for preview)
 */
export async function readFontFile(path: string): Promise<Uint8Array> {
    const invoke = getInvoke();
    if (!invoke) {
        throw new Error('Tauri required for font file reading');
    }

    const bytes = await invoke('read_font_file', { path });
    return new Uint8Array(bytes);
}

/**
 * Check if file exists
 */
export async function fileExists(path: string): Promise<boolean> {
    const invoke = getInvoke();
    if (!invoke) {
        return false;
    }

    return invoke('file_exists', { path });
}

/**
 * Get app data directory
 */
export async function getAppDataDir(): Promise<string> {
    const invoke = getInvoke();
    if (!invoke) {
        return '';
    }

    return invoke('get_app_data_dir');
}

/**
 * Check if running in Tauri desktop app
 */
export function isDesktopApp(): boolean {
    return isTauri;
}

/**
 * Open native folder picker
 */
export async function openFolderDialog(): Promise<string | null> {
    const invoke = getInvoke();
    if (!invoke) {
        return null;
    }

    // Use Tauri dialog plugin
    try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select Font Directory'
        });
        return selected as string | null;
    } catch (e) {
        console.error('Dialog plugin not available:', e);
        return null;
    }
}
