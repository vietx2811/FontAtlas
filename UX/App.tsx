import { FigmaImportHost } from './components/figma/FigmaImportHost';
import React, { useEffect, useState } from 'react';
import { getApi, type Api } from './api';
import { DetailPanel } from './components/detail/DetailPanel';
import { Library } from './components/main/Library';
import { SampleComposer } from './components/main/SampleComposer';
import { TopBar } from './components/main/TopBar';
import { ComparePage } from './components/compare/ComparePage';
import { DuplicatesPage } from './components/duplicates/DuplicatesPage';
import { PairThesePage } from './components/pair/PairThesePage';
import { PairingDesk } from './components/desk/PairingDesk';
import { Sidebar, SidebarToggle } from './components/sidebar/Sidebar';
import { WindowShell } from './components/shell/WindowShell';
import { SettingsOverlay } from './components/settings/SettingsOverlay';
import { SettingsShortcut } from './components/settings/SettingsShortcut';
import { ShortcutsHelp } from './components/help/ShortcutsHelp';
import { BootGate } from './components/boot/BootGate';
import { Splash } from './components/splash/Splash';
import { readSplashEnabled } from './components/splash/splashState';
import { NoticeStack } from './components/NoticeStack';
import { FaceActionsProvider } from './components/faces/FaceActions';
import { ActivationDialogHost } from './components/activation/ActivationDialogHost';
import { AppPickerHost } from './components/desk/AppPicker';
import { startScanSync } from './store/scan';
import { startActivationSync } from './store/activationSync';
import { useModelStore } from './store/model';
import { useUiStore } from './store/ui';
import { hydrateCardFields } from './store/cardFields';
import { t } from './i18n';

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean }> {
    state = { hasError: false };
    static getDerivedStateFromError() { return { hasError: true }; }
    componentDidCatch(error: unknown, info: unknown) { console.error('Crash caught:', error, info); }
    render() {
        if (!this.state.hasError) return this.props.children;
        return (
            <div className="min-h-screen flex items-center justify-center p-6 bg-[var(--background)]">
                <div className="max-w-md w-full bg-[var(--card)] p-8 rounded-2xl border border-[var(--border)] shadow-xl">
                    <h2 className="text-xl font-bold text-[var(--foreground)] mb-2">{t('app.errTitle')}</h2>
                    <p className="text-[var(--muted)] text-sm mb-6">{t('app.errBody')}</p>
                    <button onClick={() => window.location.reload()} className="w-full py-2 bg-[var(--foreground)] text-[var(--background)] rounded-lg font-bold hover:opacity-90 transition-all text-[11px]">
                        {t('app.restart')}
                    </button>
                </div>
            </div>
        );
    }
}

/** Thư viện: chỉ dựng sau khi backend sẵn sàng và onboarding xong (BootGate). */
const Library_: React.FC<{ api: Api }> = ({ api }) => {
    const collapsed = useUiStore(s => s.sidebarCollapsed);
    const compare = useUiStore(s => s.compare);
    const pairThese = useUiStore(s => s.pairThese);
    const desk = useUiStore(s => s.desk);
    const dups = useUiStore(s => s.view) === 'dups';
    useEffect(() => { void useUiStore.getState().hydrate(api); }, [api]);
    useEffect(() => {
        let stop: (() => void) | undefined; let dead = false;
        hydrateCardFields(api).then(s => { if (dead) s(); else stop = s; }).catch(e => console.warn('card fields hydrate failed', e));
        return () => { dead = true; stop?.(); };
    }, [api]);
    useEffect(() => {
        let stop: (() => void) | undefined; let dead = false;
        useModelStore.getState().sync(api).then(s => { if (dead) s(); else stop = s; }).catch(e => console.warn('model sync failed', e));
        return () => { dead = true; stop?.(); };
    }, [api]);

    useEffect(() => {
        let stop: (() => void) | undefined; let dead = false;
        startScanSync(api).then(s => { if (dead) s(); else stop = s; }).catch(e => console.warn('scan sync failed', e));
        return () => { dead = true; stop?.(); };
    }, [api]);

    useEffect(() => {
        let stop: (() => void) | undefined; let dead = false;
        startActivationSync(api).then(s => { if (dead) s(); else stop = s; }).catch(e => console.warn('activation sync failed', e));
        return () => { dead = true; stop?.(); };
    }, [api]);

    return (
        <FaceActionsProvider api={api}>
        <WindowShell collapsed={collapsed} sidebar={<Sidebar api={api} />} titlebarAccessory={<SidebarToggle />}>
            <div className="h-full flex text-[var(--foreground)] relative">
                {/* Thư viện và Trùng lặp giữ nguyên (ẩn) khi đang Compare/Desk/Pair these: quay lại không mất trạng thái. */}
                <div style={{ display: compare || pairThese || desk ? 'none' : 'contents' }}>
                  {dups ? <>
                    <main className="flex-1 min-w-0 h-full"><DuplicatesPage api={api} /></main>
                    <DetailPanel api={api} />
                  </> : <>
                    <main className="flex-1 min-w-0 h-full flex flex-col relative">
                        <TopBar api={api} />
                        <div className="flex-1 min-h-0"><Library api={api} /></div>
                        <SampleComposer />
                    </main>
                    <DetailPanel api={api} />
                  </>}
                </div>
                {compare && <main className="flex-1 min-w-0 h-full"><ComparePage api={api} faces={compare} /></main>}
                {desk && <main className="flex-1 min-w-0 h-full"><PairingDesk api={api} desk={desk} /></main>}
                {pairThese && <main className="flex-1 min-w-0 h-full"><PairThesePage api={api} faces={pairThese} /></main>}
                <SettingsOverlay api={api} onRerunOnboarding={() => location.reload()} />
            </div>
        </WindowShell>
        <ActivationDialogHost api={api} />
        <AppPickerHost api={api} />
        <FigmaImportHost api={api} />
        </FaceActionsProvider>
    );
};

/** Splash chỉ một lần mỗi phiên (cả khi StrictMode dựng hai lần); tắt trong test để không che giao diện. */
let splashShown = false;
const SPLASH_ON = import.meta.env.MODE !== 'test';
const devSplash = (): { hold: boolean } | null => {
    const v = import.meta.env.DEV ? new URLSearchParams(location.hash.replace(/^#/, '')).get('splash') : null; // #splash=hold: giữ splash để chụp (dev)
    return v ? { hold: v === 'hold' } : null;
};

const App: React.FC = () => {
    const api = getApi();
    const [splash, setSplash] = useState(() => SPLASH_ON && !splashShown);
    // Công tắc Settings → Chung "Hiện splash khi mở app": đang tắt thì đóng ngay (giữa lúc đọc cài đặt splash đã hiện vài khung).
    useEffect(() => { if (splash) void readSplashEnabled(api).then(on => { if (!on) { splashShown = true; setSplash(false); } }); }, []); // eslint-disable-line react-hooks/exhaustive-deps
    return (
        <ErrorBoundary>
            {splash && <Splash api={api} hold={devSplash()?.hold} onDone={() => { splashShown = true; setSplash(false); }} />}
            {/* Phần dưới inert + ẩn với trình đọc màn hình khi splash hiện: không focus, không phím, không đọc nhầm. */}
            <div style={{ display: 'contents' }} inert={splash} aria-hidden={splash || undefined}>
                <NoticeStack />
                <SettingsShortcut />
                <ShortcutsHelp />
                <BootGate api={api}><Library_ api={api} /></BootGate>
            </div>
        </ErrorBoundary>
    );
};

export default App;
