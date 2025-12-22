/**
 * Logger utility for AI agent access
 * Intercepts console output and writes to file via Tauri
 */

import { invoke } from '@tauri-apps/api/core';

interface LogEntry {
    level: string;
    message: string;
    details?: any;
}

class AppLogger {
    private enabled = true;

    /**
     * Initialize logger - intercepts console methods
     */
    init() {
        if (typeof window === 'undefined') return;

        const originalLog = console.log;
        const originalError = console.error;
        const originalWarn = console.warn;
        const originalInfo = console.info;

        console.log = (...args: any[]) => {
            originalLog.apply(console, args);
            this.writeLog('info', args);
        };

        console.error = (...args: any[]) => {
            originalError.apply(console, args);
            this.writeLog('error', args);
        };

        console.warn = (...args: any[]) => {
            originalWarn.apply(console, args);
            this.writeLog('warn', args);
        };

        console.info = (...args: any[]) => {
            originalInfo.apply(console, args);
            this.writeLog('info', args);
        };
    }

    /**
     * Write log entry to file
     */
    private async writeLog(level: string, args: any[]) {
        if (!this.enabled) return;

        try {
            const message = args
                .map(arg => {
                    if (typeof arg === 'object') {
                        try {
                            return JSON.stringify(arg);
                        } catch {
                            return String(arg);
                        }
                    }
                    return String(arg);
                })
                .join(' ');

            const entry: LogEntry = {
                level,
                message,
            };

            // Add details if first arg is object
            if (args.length > 0 && typeof args[0] === 'object' && args[0] !== null) {
                try {
                    entry.details = JSON.parse(JSON.stringify(args[0]));
                } catch {
                    // Ignore circular references
                }
            }

            await invoke('write_log', { entry });
        } catch (error) {
            // Silently fail to avoid infinite loop
            // Could write to localStorage as fallback
        }
    }

    /**
     * Manually log a message
     */
    async log(level: string, message: string, details?: any) {
        const entry: LogEntry = { level, message, details };
        try {
            await invoke('write_log', { entry });
        } catch (error) {
            console.error('Failed to write log:', error);
        }
    }

    /**
     * Enable/disable logging
     */
    setEnabled(enabled: boolean) {
        this.enabled = enabled;
    }
}

// Export singleton instance
export const logger = new AppLogger();

// Auto-initialize in browser
if (typeof window !== 'undefined') {
    logger.init();
}
