/**
 * Collections Manager Component
 * ==============================
 * 
 * Manage virtual font folders (collections)
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Folder, Plus, Trash2, Edit2, Check, X, FolderOpen } from 'lucide-react';

// Check if Tauri is available
const isTauri = typeof window !== 'undefined' && (window as any).__TAURI__;

export interface Collection {
    id: string;
    name: string;
    color: string | null;
    created_at: string;
    fontCount?: number;
}

interface CollectionsManagerProps {
    onSelectCollection?: (collection: Collection | null) => void;
    selectedCollectionId?: string | null;
}

const COLLECTION_COLORS = [
    '#e63946', '#f4a261', '#2a9d8f', '#457b9d', '#9b5de5',
    '#f72585', '#4cc9f0', '#80ed99', '#ffd60a', '#ff6b6b'
];

export const CollectionsManager: React.FC<CollectionsManagerProps> = ({
    onSelectCollection,
    selectedCollectionId
}) => {
    const [collections, setCollections] = useState<Collection[]>([]);
    const [isCreating, setIsCreating] = useState(false);
    const [newName, setNewName] = useState('');
    const [newColor, setNewColor] = useState(COLLECTION_COLORS[0]);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editName, setEditName] = useState('');

    const fetchCollections = useCallback(async () => {
        if (!isTauri) {
            // Mock data for web mode
            setCollections([
                { id: 'c1', name: 'Favorites', color: '#e63946', created_at: new Date().toISOString(), fontCount: 12 },
                { id: 'c2', name: 'Work Projects', color: '#2a9d8f', created_at: new Date().toISOString(), fontCount: 8 },
                { id: 'c3', name: 'Editorial', color: '#457b9d', created_at: new Date().toISOString(), fontCount: 5 },
            ]);
            return;
        }

        try {
            const invoke = (window as any).__TAURI__.core.invoke;
            const data = await invoke('get_collections');
            setCollections(data);
        } catch (e) {
            console.error('Failed to fetch collections:', e);
        }
    }, []);

    useEffect(() => {
        fetchCollections();
    }, [fetchCollections]);

    const handleCreate = async () => {
        if (!newName.trim()) return;

        if (isTauri) {
            try {
                const invoke = (window as any).__TAURI__.core.invoke;
                await invoke('create_collection', { name: newName, color: newColor });
                await fetchCollections();
            } catch (e) {
                console.error('Failed to create collection:', e);
            }
        } else {
            // Mock create for web
            setCollections(prev => [...prev, {
                id: `c${Date.now()}`,
                name: newName,
                color: newColor,
                created_at: new Date().toISOString(),
                fontCount: 0
            }]);
        }

        setNewName('');
        setIsCreating(false);
    };

    const handleDelete = async (id: string) => {
        if (!confirm('Delete this collection? Fonts will not be removed from library.')) return;

        if (isTauri) {
            try {
                const invoke = (window as any).__TAURI__.core.invoke;
                await invoke('delete_collection', { collection_id: id });
                await fetchCollections();
            } catch (e) {
                console.error('Failed to delete collection:', e);
            }
        } else {
            setCollections(prev => prev.filter(c => c.id !== id));
        }
    };

    const handleRename = async (id: string) => {
        if (!editName.trim()) {
            setEditingId(null);
            return;
        }

        if (isTauri) {
            try {
                const invoke = (window as any).__TAURI__.core.invoke;
                await invoke('rename_collection', { collection_id: id, name: editName });
                await fetchCollections();
            } catch (e) {
                console.error('Failed to rename collection:', e);
            }
        } else {
            setCollections(prev => prev.map(c =>
                c.id === id ? { ...c, name: editName } : c
            ));
        }

        setEditingId(null);
    };

    return (
        <div className="collections-manager">
            <div className="flex items-center justify-between mb-3">
                <h3 className="text-xs font-semibold text-stone-500 uppercase tracking-wider">
                    Collections
                </h3>
                <button
                    onClick={() => setIsCreating(true)}
                    className="p-1 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded transition-colors"
                    title="New Collection"
                >
                    <Plus size={14} />
                </button>
            </div>

            {/* Create new collection form */}
            {isCreating && (
                <div className="mb-3 p-3 bg-stone-100 rounded-lg">
                    <input
                        type="text"
                        value={newName}
                        onChange={(e) => setNewName(e.target.value)}
                        placeholder="Collection name..."
                        className="w-full px-2 py-1.5 text-sm bg-white border border-stone-200 rounded focus:outline-none focus:ring-2 focus:ring-stone-400 mb-2"
                        autoFocus
                        onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
                    />
                    <div className="flex items-center gap-1 mb-2">
                        {COLLECTION_COLORS.map(color => (
                            <button
                                key={color}
                                onClick={() => setNewColor(color)}
                                className={`w-5 h-5 rounded-full transition-transform ${newColor === color ? 'scale-125 ring-2 ring-offset-1 ring-stone-400' : ''}`}
                                style={{ backgroundColor: color }}
                            />
                        ))}
                    </div>
                    <div className="flex gap-2">
                        <button
                            onClick={handleCreate}
                            className="flex-1 px-3 py-1.5 text-xs font-medium bg-stone-900 text-white rounded hover:bg-stone-800 transition-colors"
                        >
                            Create
                        </button>
                        <button
                            onClick={() => setIsCreating(false)}
                            className="px-3 py-1.5 text-xs text-stone-500 hover:text-stone-700"
                        >
                            Cancel
                        </button>
                    </div>
                </div>
            )}

            {/* Collections list */}
            <div className="space-y-1">
                {/* All Fonts option */}
                <button
                    onClick={() => onSelectCollection?.(null)}
                    className={`w-full flex items-center gap-2 px-3 py-2 text-sm rounded-lg transition-colors ${!selectedCollectionId
                            ? 'bg-stone-900 text-white'
                            : 'text-stone-600 hover:bg-stone-100'
                        }`}
                >
                    <FolderOpen size={16} />
                    <span className="flex-1 text-left">All Fonts</span>
                </button>

                {collections.map(collection => (
                    <div
                        key={collection.id}
                        className={`group flex items-center gap-2 px-3 py-2 rounded-lg transition-colors ${selectedCollectionId === collection.id
                                ? 'bg-stone-200'
                                : 'hover:bg-stone-100'
                            }`}
                    >
                        <div
                            className="w-3 h-3 rounded-full flex-shrink-0"
                            style={{ backgroundColor: collection.color || '#888' }}
                        />

                        {editingId === collection.id ? (
                            <div className="flex-1 flex items-center gap-1">
                                <input
                                    type="text"
                                    value={editName}
                                    onChange={(e) => setEditName(e.target.value)}
                                    className="flex-1 px-1 py-0.5 text-sm bg-white border border-stone-300 rounded"
                                    autoFocus
                                    onKeyDown={(e) => {
                                        if (e.key === 'Enter') handleRename(collection.id);
                                        if (e.key === 'Escape') setEditingId(null);
                                    }}
                                />
                                <button onClick={() => handleRename(collection.id)} className="p-1 text-green-600 hover:bg-green-50 rounded">
                                    <Check size={12} />
                                </button>
                                <button onClick={() => setEditingId(null)} className="p-1 text-stone-400 hover:bg-stone-50 rounded">
                                    <X size={12} />
                                </button>
                            </div>
                        ) : (
                            <>
                                <button
                                    onClick={() => onSelectCollection?.(collection)}
                                    className="flex-1 text-left text-sm text-stone-700 truncate"
                                >
                                    {collection.name}
                                </button>
                                <span className="text-xs text-stone-400">
                                    {collection.fontCount || 0}
                                </span>
                                <div className="hidden group-hover:flex items-center gap-1">
                                    <button
                                        onClick={() => {
                                            setEditingId(collection.id);
                                            setEditName(collection.name);
                                        }}
                                        className="p-1 text-stone-400 hover:text-stone-600 rounded"
                                    >
                                        <Edit2 size={12} />
                                    </button>
                                    <button
                                        onClick={() => handleDelete(collection.id)}
                                        className="p-1 text-stone-400 hover:text-red-500 rounded"
                                    >
                                        <Trash2 size={12} />
                                    </button>
                                </div>
                            </>
                        )}
                    </div>
                ))}
            </div>
        </div>
    );
};

export default CollectionsManager;
