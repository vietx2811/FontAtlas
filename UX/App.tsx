import React, { useEffect } from 'react';
import { getApi } from './api';
import { Library } from './components/main/Library';
import { WindowShell } from './components/shell/WindowShell';
import { useUiStore, type View } from './store/ui';

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean }> {
    state = { hasError: false };
    static getDerivedStateFromError() { return { hasError: true }; }
    componentDidCatch(error: unknown, info: unknown) { console.error('Crash caught:', error, info); }
    render() {
        if (!this.state.hasError) return this.props.children;
        return (
            <div className="min-h-screen flex items-center justify-center p-6 bg-[var(--background)]">
                <div className="max-w-md w-full bg-[var(--card)] p-8 rounded-2xl border border-[var(--border)] shadow-xl">
                    <h2 className="text-xl font-bold text-[var(--foreground)] mb-2">Lỗi giao diện</h2>
                    <p className="text-[var(--muted)] text-sm mb-6">Giao diện vừa gặp sự cố. Dữ liệu của bạn không bị ảnh hưởng.</p>
                    <button onClick={() => window.location.reload()} className="w-full py-2 bg-[var(--foreground)] text-[var(--background)] rounded-lg font-bold hover:opacity-90 transition-all text-[11px]">
                        Khởi động lại
                    </button>
                </div>
            </div>
        );
    }
}

// Tạm thời cho đến T4 (Sidebar thật).
const NAV: Array<[View, string]> = [['all', 'Tất cả'], ['star', 'Yêu thích'], ['recent', 'Gần đây'], ['active', 'Đang bật']];

const App: React.FC = () => {
    const api = getApi();
    const view = useUiStore(s => s.view);
    const setView = useUiStore(s => s.setView);
    useEffect(() => { void useUiStore.getState().hydrate(api); }, [api]);

    const sidebar = (
        <nav className="p-2 flex flex-col gap-1">
            {NAV.map(([v, label]) => (
                <button key={v} onClick={() => setView(v)} aria-current={view === v} className="text-left px-3 py-1.5 rounded-md text-sm hover:bg-[var(--chrome-hover)] aria-[current=true]:bg-[var(--chrome-active)]">{label}</button>
            ))}
        </nav>
    );

    return (
        <ErrorBoundary>
            <WindowShell sidebar={sidebar}>
                <main className="h-full text-[var(--foreground)]"><Library api={api} /></main>
            </WindowShell>
        </ErrorBoundary>
    );
};

export default App;
