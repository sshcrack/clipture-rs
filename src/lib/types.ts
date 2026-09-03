export type CaptureSourceKind = "auto" | "game" | "window" | "monitor";

export interface CaptureSource {
  id: string;
  kind: CaptureSourceKind;
  label: string;
  subtitle?: string;
  pid?: number;
  isGame?: boolean;
  nativeId?: string;
}

export interface PendingClip {
  id: string;
  requestedAt: number;
  savesAt: number;
}

export interface CaptureSnapshot {
  phase: "booting" | "idle" | "buffering" | "recording" | "error";
  replayActive: boolean;
  recordingActive: boolean;
  recordingStartedAt?: number;
  source?: CaptureSource;
  pendingClips: PendingClip[];
  encoder?: string;
  fps: number;
  outputResolution: string;
  message?: string;
}

export interface AppSettings {
  replay: {
    enabled: boolean;
    autoStart: boolean;
    preRollSeconds: number;
    postRollSeconds: number;
    maxSizeMb: number;
  };
  recording: {
    autoRecordGames: boolean;
    quality: "performance" | "balanced" | "quality" | "custom";
    fps: number;
    width: number;
    height: number;
    bitrateKbps: number;
  };
  audio: {
    applicationAudio: boolean;
    microphone: boolean;
    microphoneDeviceId?: string;
  };
  hotkeys: {
    saveClip: string;
    toggleRecording: string;
  };
  notifications: {
    desktop: boolean;
    sound: boolean;
    inApp: boolean;
  };
  storage: {
    root: string;
    recordingBudgetGb?: number;
  };
  behavior: {
    startWithSystem: boolean;
    closeToTray: boolean;
    closeBehaviorConfirmed: boolean;
  };
  advanced: {
    encoder?: string;
    captureMethod: "auto" | "game" | "window" | "monitor";
  };
  onboardingComplete: boolean;
}

export interface LibraryItem {
  id: string;
  kind: "clip" | "recording";
  title: string;
  game: string;
  createdAt: number;
  durationMs?: number;
  sizeBytes: number;
  path: string;
  protected: boolean;
}

export interface AppSnapshot {
  capture: CaptureSnapshot;
  settings: AppSettings;
  recent: LibraryItem[];
  version: string;
}

export interface TrimRequest {
  id: string;
  startSeconds: number;
  endSeconds: number;
  title?: string;
}


export interface ObsOption {
  label: string;
  value: string;
  disabled: boolean;
}

export interface ObsPropertySchema {
  name: string;
  label: string;
  description?: string;
  kind: string;
  enabled: boolean;
  visible: boolean;
  options: ObsOption[];
}

export interface ObsEncoderCapability {
  id: string;
  label: string;
  codec?: string;
  hardware: boolean;
  properties: ObsPropertySchema[];
}

export interface ObsSourceCapability {
  id: string;
  label: string;
  properties: ObsPropertySchema[];
}

export interface ObsCapabilitiesSnapshot {
  videoEncoders: ObsEncoderCapability[];
  captureSources: ObsSourceCapability[];
  microphones: ObsOption[];
}
