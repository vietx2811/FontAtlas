/**
 * Preferences Panel Component
 * ============================
 * 
 * Settings modal for app preferences including Qdrant database configuration
 */

import React, { useState, useEffect } from 'react';
import { X, Database, Cloud, HardDrive, Check, AlertCircle } from 'lucide-react';
import { useSettings, type AppSettings } from '../hooks/useSettings';
import { clearLibrary } from '../services/library';
import { Loader2 } from 'lucide-react';

interface PreferencesPanelProps {
    isOpen: boolean;
    onClose: () => void;
}

export const PreferencesPanel: React.FC<PreferencesPanelProps> = ({ isOpen, onClose }) => {
    const { settings, saveSettings } = useSettings();
    const [localSettings, setLocalSettings] = useState<AppSettings>(settings);
    const [connectionStatus, setConnectionStatus] = useState<'idle' | 'testing' | 'connected' | 'error'>('idle');
    const [isClearing, setIsClearing] = useState(false);

    // Sync local state when settings change
    useEffect(() => {
        setLocalSettings(settings);
    }, [settings]);

    const handleSave = () => {
        saveSettings(localSettings);
        onClose();
    };

    const handleModeChange = (mode: 'local' | 'remote') => {
        setLocalSettings(prev => ({ ...prev, qdrantMode: mode }));
        setConnectionStatus('idle');
    };

    const testConnection = async () => {
        setConnectionStatus('testing');
        const url = localSettings.qdrantMode === 'local'
            ? localSettings.qdrantLocalUrl
            : localSettings.qdrantRemoteUrl;

        try {
            // Qdrant returns version info at root endpoint
            const response = await fetch(url, {
                method: 'GET',
                mode: 'cors',
            });
            if (response.ok) {
                const data = await response.json();
                // Qdrant returns { title: "qdrant - vector search engine", version: "x.x.x", ... }
                if (data.title && data.title.includes('qdrant')) {
                    setConnectionStatus('connected');
                } else {
                    setConnectionStatus('error');
                }
            } else {
                setConnectionStatus('error');
            }
        } catch (e) {
            console.error('Connection test failed:', e);
            setConnectionStatus('error');
        }
    };

    const handleClearLibrary = async () => {
        if (!confirm('Clear all fonts from library? This will remove all scanned fonts. The font files will NOT be deleted.')) {
            return;
        }

        setIsClearing(true);
        try {
            const deleted = await clearLibrary();
            console.log(`Cleared ${deleted} fonts from library`);
            // We should ideally callback to refresh app state, but localStorage clear will handle Directories
            // For now, reloading or refetching via context would be best, but simple confirm is okay
            window.location.reload(); // Simple brute force update for now to clear all state
        } catch (e) {
            console.error('Failed to clear library:', e);
            alert('Failed to clear library');
        } finally {
            setIsClearing(false);
        }
    };

    if (!isOpen) return null;

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40"
                onClick={onClose}
            />

            {/* Modal */}
            <div className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-md bg-white rounded-2xl shadow-2xl z-50 overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between px-6 py-4 border-b border-stone-200">
                    <h2 className="text-lg font-semibold text-stone-900 flex items-center gap-2">
                        <Database size={20} className="text-stone-500" />
                        Preferences
                    </h2>
                    <button
                        onClick={onClose}
                        className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-lg transition-colors"
                    >
                        <X size={20} />
                    </button>
                </div>

                {/* Content */}
                <div className="p-6 space-y-6">
                    {/* Vector Database Section */}
                    <div>
                        <h3 className="text-sm font-medium text-stone-700 mb-3">Vector Database</h3>
                        <div className="space-y-2">
                            {/* Local Option */}
                            <button
                                onClick={() => handleModeChange('local')}
                                className={`w-full flex items-center gap-3 p-4 rounded-xl border-2 transition-all ${localSettings.qdrantMode === 'local'
                                    ? 'border-stone-900 bg-stone-50'
                                    : 'border-stone-200 hover:border-stone-300'
                                    }`}
                            >
                                <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${localSettings.qdrantMode === 'local' ? 'bg-stone-900 text-white' : 'bg-stone-100 text-stone-500'
                                    }`}>
                                    <HardDrive size={20} />
                                </div>
                                <div className="flex-1 text-left">
                                    <div className="font-medium text-stone-900">Local</div>
                                    <div className="text-xs text-stone-500">localhost:6333</div>
                                </div>
                                {localSettings.qdrantMode === 'local' && (
                                    <Check size={20} className="text-stone-900" />
                                )}
                            </button>

                            {/* Remote Option */}
                            <button
                                onClick={() => handleModeChange('remote')}
                                className={`w-full flex items-center gap-3 p-4 rounded-xl border-2 transition-all ${localSettings.qdrantMode === 'remote'
                                    ? 'border-amber-500 bg-amber-50'
                                    : 'border-stone-200 hover:border-stone-300'
                                    }`}
                            >
                                <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${localSettings.qdrantMode === 'remote' ? 'bg-amber-500 text-white' : 'bg-stone-100 text-stone-500'
                                    }`}>
                                    <Cloud size={20} />
                                </div>
                                <div className="flex-1 text-left">
                                    <div className="font-medium text-stone-900">Remote</div>
                                    <div className="text-xs text-stone-500">vectordb.vietx.xyz</div>
                                </div>
                                {localSettings.qdrantMode === 'remote' && (
                                    <Check size={20} className="text-amber-600" />
                                )}
                            </button>
                        </div>
                    </div>

                    {/* Connection Status */}
                    <div className="flex items-center justify-between p-3 bg-stone-50 rounded-lg">
                        <div className="flex items-center gap-2">
                            {connectionStatus === 'idle' && (
                                <span className="w-2 h-2 rounded-full bg-stone-300"></span>
                            )}
                            {connectionStatus === 'testing' && (
                                <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse"></span>
                            )}
                            {connectionStatus === 'connected' && (
                                <span className="w-2 h-2 rounded-full bg-green-500"></span>
                            )}
                            {connectionStatus === 'error' && (
                                <AlertCircle size={14} className="text-red-500" />
                            )}
                            <span className="text-sm text-stone-600">
                                {connectionStatus === 'idle' && 'Not tested'}
                                {connectionStatus === 'testing' && 'Testing...'}
                                {connectionStatus === 'connected' && 'Connected'}
                                {connectionStatus === 'error' && 'Connection failed'}
                            </span>
                        </div>
                        <button
                            onClick={testConnection}
                            disabled={connectionStatus === 'testing'}
                            className="text-xs font-medium text-stone-600 hover:text-stone-900 disabled:opacity-50"
                        >
                            Test Connection
                        </button>
                    </div>

                    {/* Library Management */}
                    <div>
                        <h3 className="text-sm font-medium text-stone-700 mb-3">Library Management</h3>
                        <div className="p-4 bg-red-50 rounded-xl border border-red-100">
                            <h4 className="text-sm font-medium text-red-900 mb-1">Danger Zone</h4>
                            <p className="text-xs text-red-700 mb-3">
                                Clear all fonts and directories from the FontAtlas database. Font files on your disk will not be touched.
                            </p>
                            <button
                                onClick={handleClearLibrary}
                                disabled={isClearing}
                                className="px-3 py-2 text-xs font-bold text-white bg-red-600 hover:bg-red-700 rounded-lg transition-colors flex items-center gap-2 disabled:opacity-50"
                            >
                                {isClearing ? <Loader2 size={12} className="animate-spin" /> : <Database size={12} />}
                                Clear Entire Library
                            </button>
                        </div>
                    </div>
                </div>

                {/* Footer */}
                <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-stone-100 bg-stone-50">
                    <button
                        onClick={onClose}
                        className="px-4 py-2 text-sm font-medium text-stone-600 hover:text-stone-900 hover:bg-stone-200 rounded-lg transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        onClick={handleSave}
                        className="px-4 py-2 text-sm font-medium text-white bg-stone-900 hover:bg-stone-800 rounded-lg transition-colors"
                    >
                        Save Changes
                    </button>
                </div>
            </div>
        </>
    );
};

export default PreferencesPanel;
