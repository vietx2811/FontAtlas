import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

export interface DeskFont {
    id: string;
    name: string;
    file_path: string;
    primary_class?: string;
    added_at: number;
}

interface DeskState {
    items: DeskFont[];
    addItem: (font: Omit<DeskFont, 'added_at'>) => void;
    removeItem: (id: string) => void;
    clearDesk: () => void;
    isInDesk: (id: string) => boolean;
}

export const useDeskStore = create<DeskState>()(
    persist(
        (set, get) => ({
            items: [],
            addItem: (font) => {
                const items = get().items;
                if (!items.find(item => item.id === font.id)) {
                    set({ items: [...items, { ...font, added_at: Date.now() }] });
                }
            },
            removeItem: (id) => {
                set({ items: get().items.filter(item => item.id !== id) });
            },
            clearDesk: () => set({ items: [] }),
            isInDesk: (id) => get().items.some(item => item.id === id),
        }),
        {
            name: 'fontatlas-desk-storage',
            storage: createJSONStorage(() => localStorage),
        }
    )
);

export const deskService = {
    addToDesk: (font: Omit<DeskFont, 'added_at'>) => useDeskStore.getState().addItem(font),
    removeFromDesk: (id: string) => useDeskStore.getState().removeItem(id),
    getItems: () => useDeskStore.getState().items,
    isInDesk: (id: string) => useDeskStore.getState().isInDesk(id),
};
