/**
 * Settings Hook
 * ==============
 * 
 * Manages app settings with localStorage persistence
 * Uses custom events for cross-component state sync
 */

import { useState, useEffect, useCallback, useSyncExternalStore } from 'react';

export interface AppSettings {
    // Qdrant settings
    qdrantMode: 'local' | 'remote';
    qdrantLocalUrl: string;
    qdrantRemoteUrl: string;

    // Future settings can be added here
    theme?: 'light' | 'dark' | 'system';
}

const STORAGE_KEY = 'fontatlas_settings';
const SETTINGS_EVENT = 'fontatlas_settings_changed';

const DEFAULT_SETTINGS: AppSettings = {
    qdrantMode: 'local',
    qdrantLocalUrl: 'http://localhost:6333',
    qdrantRemoteUrl: 'https://vectordb.vietx.xyz',
    theme: 'light',
};

// Shared settings store for sync across components
let cachedSettings: AppSettings = DEFAULT_SETTINGS;
let listeners: Set<() => void> = new Set();

// Load from localStorage on init
try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
        cachedSettings = { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    }
} catch (e) {
    console.error('Failed to load settings:', e);
}

function getSnapshot() {
    return cachedSettings;
}

function subscribe(callback: () => void) {
    listeners.add(callback);
    return () => listeners.delete(callback);
}

function notifyListeners() {
    listeners.forEach(listener => listener());
}

export function useSettings() {
    const settings = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

    // Save settings to localStorage and notify all listeners
    const saveSettings = useCallback((newSettings: Partial<AppSettings>) => {
        cachedSettings = { ...cachedSettings, ...newSettings };
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(cachedSettings));
        } catch (e) {
            console.error('Failed to save settings:', e);
        }
        notifyListeners();
    }, []);

    // Get current Qdrant URL based on mode
    const getQdrantUrl = useCallback(() => {
        return settings.qdrantMode === 'local'
            ? settings.qdrantLocalUrl
            : settings.qdrantRemoteUrl;
    }, [settings]);

    // Reset to defaults
    const resetSettings = useCallback(() => {
        cachedSettings = DEFAULT_SETTINGS;
        try {
            localStorage.removeItem(STORAGE_KEY);
        } catch (e) {
            console.error('Failed to reset settings:', e);
        }
        notifyListeners();
    }, []);

    return {
        settings,
        isLoaded: true,
        saveSettings,
        getQdrantUrl,
        resetSettings,
    };
}

export default useSettings;
