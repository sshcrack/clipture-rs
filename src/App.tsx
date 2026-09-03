import { useCallback, useEffect, useState } from "react";
import { Onboarding } from "./components/Onboarding";
import { Sidebar, type Page } from "./components/Sidebar";
import { ToastHost, type ToastMessage } from "./components/ToastHost";
import { desktop } from "./lib/desktop";
import type {
  AppSettings,
  AppSnapshot,
  CaptureSource,
  LibraryItem,
  ObsCapabilitiesSnapshot,
} from "./lib/types";
import { HomePage } from "./pages/HomePage";
import { LibraryPage } from "./pages/LibraryPage";
import { SettingsPage } from "./pages/SettingsPage";

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [snapshot, setSnapshot] = useState<AppSnapshot>();
  const [sources, setSources] = useState<CaptureSource[]>([]);
  const [items, setItems] = useState<LibraryItem[]>([]);
  const [capabilities, setCapabilities] = useState<ObsCapabilitiesSnapshot>();
  const [error, setError] = useState<string>();
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const refresh = useCallback(async () => {
    try {
      const [next, allSources, library] = await Promise.all([
        desktop.snapshot(),
        desktop.sources(),
        desktop.library(),
      ]);
      setSnapshot(next);
      setSources(allSources);
      setItems(library);
      setError(undefined);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const refreshCapabilities = useCallback(async () => {
    try {
      setCapabilities(await desktop.obsCapabilities());
    } catch {
      setCapabilities(undefined);
    }
  }, []);

  useEffect(() => {
    refresh();
    refreshCapabilities();
    const timer = window.setInterval(refresh, 3000);
    let offCapture: (() => void) | undefined;
    let offLibrary: (() => void) | undefined;
    let offFeedback: (() => void) | undefined;

    desktop.onCaptureChanged((capture) => setSnapshot((current) => current ? { ...current, capture } : current)).then((off) => offCapture = off);
    desktop.onLibraryChanged(refresh).then((off) => offLibrary = off);
    desktop.onFeedback((message) => {
      const toast = { id: crypto.randomUUID(), ...message };
      setToasts((current) => [...current.slice(-2), toast]);
      window.setTimeout(
        () => setToasts((current) => current.filter((item) => item.id !== toast.id)),
        3200,
      );
    }).then((off) => offFeedback = off);

    return () => {
      window.clearInterval(timer);
      offCapture?.();
      offLibrary?.();
      offFeedback?.();
    };
  }, [refresh, refreshCapabilities]);

  if (!snapshot) {
    return <div className="boot-screen"><div className="brand-mark loading" /><strong>Starting Clipture</strong><span>{error ?? "Preparing capture engine…"}</span></div>;
  }

  const updateSettings = async (settings: AppSettings) => {
    const saved = await desktop.updateSettings(settings);
    setSnapshot((current) => current ? { ...current, settings: saved } : current);
    await refreshCapabilities();
  };

  return <div className="app-shell">
    <Sidebar
      page={page}
      setPage={setPage}
      captureActive={snapshot.capture.replayActive || snapshot.capture.recordingActive}
    />
    <div className="content-shell">
      {page === "home" && <HomePage
        capture={snapshot.capture}
        settings={snapshot.settings}
        recent={items}
        sources={sources}
        onSaveClip={async () => { await desktop.saveClip(); await refresh(); }}
        onToggleReplay={async () => { await desktop.toggleReplay(); await refresh(); }}
        onToggleRecording={async () => { await desktop.toggleRecording(); await refresh(); }}
        onSelectSource={async (source) => { await desktop.selectSource(source); await refresh(); }}
        onOpenLibrary={() => setPage("library")}
        previewEnabled={snapshot.settings.onboardingComplete}
      />}
      {page === "library" && <LibraryPage
        items={items}
        onReveal={desktop.reveal}
        onDelete={async (id) => { await desktop.remove(id); await refresh(); }}
        onProtect={desktop.protect}
        onRefresh={refresh}
      />}
      {page === "settings" && <SettingsPage settings={snapshot.settings} capabilities={capabilities} onChooseFolder={desktop.chooseFolder} onSave={updateSettings} />}
    </div>
    <ToastHost toasts={toasts} dismiss={(id) => setToasts((current) => current.filter((item) => item.id !== id))} />
    {!snapshot.settings.onboardingComplete && <Onboarding
      settings={snapshot.settings}
      capabilities={capabilities}
      onChooseFolder={desktop.chooseFolder}
      onComplete={async (settings) => { await desktop.completeOnboarding(settings); await refresh(); await refreshCapabilities(); }}
    />}
  </div>;
}
