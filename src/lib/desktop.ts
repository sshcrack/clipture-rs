import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppSettings, AppSnapshot, CaptureSnapshot, CaptureSource, LibraryItem, ObsCapabilitiesSnapshot, TrimRequest } from "./types";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const now = Date.now();
let mockSettings: AppSettings = {
  replay: { enabled: true, autoStart: true, preRollSeconds: 30, postRollSeconds: 10, maxSizeMb: 1500 },
  recording: { autoRecordGames: false, quality: "balanced", fps: 60, width: 1920, height: 1080, bitrateKbps: 12000 },
  audio: { applicationAudio: true, microphone: true },
  hotkeys: { saveClip: "F9", toggleRecording: "Ctrl+Shift+R" },
  notifications: { desktop: true, sound: true, inApp: true },
  storage: { root: "Videos/Clipture", recordingBudgetGb: 100 },
  behavior: { startWithSystem: false, closeToTray: true, closeBehaviorConfirmed: false },
  advanced: { encoder: undefined, captureMethod: "auto" },
  onboardingComplete: true,
};
const mockRecent: LibraryItem[] = [
  { id: "1", kind: "clip", title: "Counter-Strike 2 · 22:41", game: "Counter-Strike 2", createdAt: now - 11 * 60_000, durationMs: 40_000, sizeBytes: 58_000_000, path: "Videos/Clipture/Counter-Strike 2/2026-09-03/22-41-clip.mp4", protected: false },
  { id: "2", kind: "recording", title: "Counter-Strike 2 · Session", game: "Counter-Strike 2", createdAt: now - 68 * 60_000, durationMs: 2_740_000, sizeBytes: 4_820_000_000, path: "Videos/Clipture/Counter-Strike 2/2026-09-03/session-2142.mkv", protected: false },
  { id: "3", kind: "clip", title: "VALORANT · 19:08", game: "VALORANT", createdAt: now - 25 * 60 * 60_000, durationMs: 40_000, sizeBytes: 64_000_000, path: "Videos/Clipture/VALORANT/2026-09-02/19-08-clip.mp4", protected: true },
];
const mockCapabilities: ObsCapabilitiesSnapshot = {
  videoEncoders: [
    { id: "ffmpeg_nvenc", label: "NVIDIA NVENC H.264", codec: "h264", hardware: true, properties: [
      { name: "bitrate", label: "Bitrate", kind: "Integer", enabled: true, visible: true, options: [] },
      { name: "preset", label: "Preset", kind: "List", enabled: true, visible: true, options: [{ label: "Quality", value: "p5", disabled: false }] },
    ] },
    { id: "obs_x264", label: "x264", codec: "h264", hardware: false, properties: [] },
  ],
  captureSources: [
    { id: "game_capture", label: "Game Capture", properties: [
      { name: "capture_cursor", label: "Capture Cursor", kind: "Boolean", enabled: true, visible: true, options: [] },
    ] },
  ],
  microphones: [
    { label: "System default microphone", value: "default", disabled: false },
    { label: "Microphone (USB Audio)", value: "usb-mic", disabled: false },
  ],
};

let mockCapture: CaptureSnapshot = {
  phase: "buffering", replayActive: true, recordingActive: false, pendingClips: [], encoder: "NVIDIA NVENC H.264", fps: 60, outputResolution: "1920 × 1080",
  source: { id: "game:14284", kind: "game", label: "Counter-Strike 2", subtitle: "Game capture · cs2.exe", pid: 14284, isGame: true },
};

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) throw new Error(`Mock handler missing for ${command}`);
  return invoke<T>(command, args);
}

let previewQueue: Promise<unknown> = Promise.resolve();

export const desktop = {
  async snapshot(): Promise<AppSnapshot> {
    if (!isTauri) return { capture: mockCapture, settings: mockSettings, recent: mockRecent, version: "0.1.0-dev" };
    return call("app_snapshot");
  },
  async saveClip(): Promise<void> {
    if (!isTauri) {
      const requestedAt = Date.now();
      mockCapture = { ...mockCapture, pendingClips: [...mockCapture.pendingClips, { id: crypto.randomUUID(), requestedAt, savesAt: requestedAt + mockSettings.replay.postRollSeconds * 1000 }] };
      return;
    }
    return call("save_clip");
  },
  async toggleRecording(): Promise<void> {
    if (!isTauri) {
      mockCapture = { ...mockCapture, recordingActive: !mockCapture.recordingActive, recordingStartedAt: mockCapture.recordingActive ? undefined : Date.now(), phase: mockCapture.recordingActive ? "buffering" : "recording" };
      return;
    }
    return call("toggle_recording");
  },
  async toggleReplay(): Promise<void> {
    if (!isTauri) { mockCapture = { ...mockCapture, replayActive: !mockCapture.replayActive, phase: mockCapture.replayActive ? "idle" : "buffering" }; return; }
    return call("toggle_replay");
  },
  async obsCapabilities(): Promise<ObsCapabilitiesSnapshot> {
    return isTauri ? call("obs_capabilities") : mockCapabilities;
  },
  async updatePreview(bounds: { x: number; y: number; width: number; height: number; visible: boolean }): Promise<boolean> {
    if (!isTauri) return false;
    const request = previewQueue.then(() => call<boolean>("update_preview", bounds));
    previewQueue = request.catch(() => false);
    return request;
  },
  async sources(): Promise<CaptureSource[]> {
    if (!isTauri) return [
      { id: "auto", kind: "auto", label: "Automatic", subtitle: "Prefer detected games" },
      mockCapture.source!,
      { id: "window:491", kind: "window", label: "Firefox", subtitle: "Window capture · firefox.exe", pid: 491 },
      { id: "monitor:0", kind: "monitor", label: "Display 1", subtitle: "2560 × 1440" },
    ];
    return call("list_sources");
  },
  async selectSource(source: CaptureSource): Promise<void> {
    if (!isTauri) { mockCapture = { ...mockCapture, source }; return; }
    return call("select_source", { source });
  },
  async chooseFolder(defaultPath?: string): Promise<string | undefined> {
    if (!isTauri) return undefined;
    const selected = await open({ directory: true, multiple: false, defaultPath });
    return typeof selected === "string" ? selected : undefined;
  },
  async settings(): Promise<AppSettings> { return isTauri ? call("get_settings") : structuredClone(mockSettings); },
  async updateSettings(settings: AppSettings): Promise<AppSettings> {
    if (!isTauri) { mockSettings = structuredClone(settings); return mockSettings; }
    return call("update_settings", { settings });
  },
  async library(): Promise<LibraryItem[]> { return isTauri ? call("library_list") : mockRecent; },
  async rename(id: string, title: string): Promise<void> { if (isTauri) return call("library_rename", { id, title }); },
  async remove(id: string): Promise<void> { if (isTauri) return call("library_delete", { id }); },
  async protect(id: string, protectedValue: boolean): Promise<void> { if (isTauri) return call("library_protect", { id, protectedValue }); },
  async trim(request: TrimRequest): Promise<LibraryItem> { return call("trim_clip", { request }); },
  async reveal(id: string): Promise<void> { if (isTauri) return call("library_reveal", { id }); },
  async completeOnboarding(settings: AppSettings): Promise<void> { if (isTauri) return call("complete_onboarding", { settings }); mockSettings = { ...settings, onboardingComplete: true }; },
  async onCaptureChanged(handler: (snapshot: CaptureSnapshot) => void): Promise<UnlistenFn> {
    if (!isTauri) return () => undefined;
    return listen<CaptureSnapshot>("capture://state", event => handler(event.payload));
  },
  async onLibraryChanged(handler: () => void): Promise<UnlistenFn> {
    if (!isTauri) return () => undefined;
    return listen("library://changed", handler);
  },
  async onFeedback(handler: (message: { title: string; body: string }) => void): Promise<UnlistenFn> {
    if (!isTauri) return () => undefined;
    return listen<{ title: string; body: string }>("feedback://toast", event => handler(event.payload));
  },
};
