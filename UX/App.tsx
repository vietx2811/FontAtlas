import React, { useEffect } from 'react';
import { getApi } from './api';
import { Library } from './components/main/Library';
import { SampleComposer } from './components/main/SampleComposer';
import { TopBar } from './components/main/TopBar';
import { Sidebar, SidebarToggle } from './components/sidebar/Sidebar';
import { WindowShell } from './components/shell/WindowShell';
import { useUiStore } from './store/ui';

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

const App: React.FC = () => {
    const api = getApi();
    const collapsed = useUiStore(s => s.sidebarCollapsed);
    useEffect(() => { void useUiStore.getState().hydrate(api); }, [api]);

    return (
        <ErrorBoundary>
            <WindowShell collapsed={collapsed} sidebar={<Sidebar api={api} />} titlebarAccessory={<SidebarToggle />}>
                <main className="h-full flex flex-col relative text-[var(--foreground)]">
                    <TopBar api={api} />
                    <div className="flex-1 min-h-0"><Library api={api} /></div>
                    <SampleComposer />
                </main>
            </WindowShell>
        </ErrorBoundary>
    );
};

export default App;
