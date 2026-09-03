import { useEffect, useState } from "react";
import {
  AudioLines,
  Cpu,
  HardDrive,
  Keyboard,
  MonitorCog,
  Save,
  Settings2,
} from "lucide-react";
import type {
  AppSettings,
  ObsCapabilitiesSnapshot,
  ObsPropertySchema,
} from "../lib/types";

type Section = "capture" | "audio" | "hotkeys" | "storage" | "behavior" | "advanced";
type Quality = AppSettings["recording"]["quality"];

const sections: { id: Section; label: string; icon: React.ReactNode }[] = [
  { id: "capture", label: "Capture", icon: <MonitorCog size={16} /> },
  { id: "audio", label: "Audio", icon: <AudioLines size={16} /> },
  { id: "hotkeys", label: "Hotkeys", icon: <Keyboard size={16} /> },
  { id: "storage", label: "Storage", icon: <HardDrive size={16} /> },
  { id: "behavior", label: "Behavior", icon: <Settings2 size={16} /> },
  { id: "advanced", label: "Advanced", icon: <Cpu size={16} /> },
];

const qualityPresets: Record<Exclude<Quality, "custom">, Partial<AppSettings["recording"]>> = {
  performance: { fps: 60, width: 1280, height: 720, bitrateKbps: 6000 },
  balanced: { fps: 60, width: 1920, height: 1080, bitrateKbps: 12000 },
  quality: { fps: 60, width: 1920, height: 1080, bitrateKbps: 20000 },
};

export function SettingsPage({
  settings,
  capabilities,
  onChooseFolder,
  onSave,
}: {
  settings: AppSettings;
  capabilities?: ObsCapabilitiesSnapshot;
  onChooseFolder: (defaultPath?: string) => Promise<string | undefined>;
  onSave: (settings: AppSettings) => Promise<void>;
}) {
  const [draft, setDraft] = useState(settings);
  const [section, setSection] = useState<Section>("capture");
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState<string>();

  useEffect(() => setDraft(settings), [settings]);

  const update = (key: keyof AppSettings, patch: Record<string, unknown>) =>
    setDraft((current) => ({
      ...current,
      [key]: { ...(current[key] as object), ...patch },
    }) as AppSettings);

  const updateRecording = (patch: Partial<AppSettings["recording"]>, custom = true) =>
    update("recording", { ...patch, ...(custom ? { quality: "custom" } : {}) });

  const selectQuality = (quality: Quality) => {
    if (quality === "custom") {
      update("recording", { quality });
      return;
    }
    update("recording", { quality, ...qualityPresets[quality] });
  };

  const save = async () => {
    setSaving(true);
    setSaveError(undefined);
    try {
      await onSave(draft);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1500);
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  const microphones = capabilities?.microphones ?? [];
  const currentMicrophone = draft.audio.microphoneDeviceId ?? "default";

  return (
    <main className="page settings-page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Preferences</p>
          <h1>Set it once. Forget about it.</h1>
          <p className="header-note">Clipture keeps OBS complexity out of the way until you ask for it.</p>
        </div>
        <button className="primary-button" onClick={save} disabled={saving}>
          <Save size={16} />
          {saving ? "Saving…" : saved ? "Saved" : "Save changes"}
        </button>
      </header>

      {saveError && <div className="error-banner"><strong>Settings were not applied</strong><span>{saveError}</span></div>}

      <div className="settings-shell">
        <nav className="settings-nav">
          {sections.map((item) => (
            <button
              key={item.id}
              className={section === item.id ? "active" : ""}
              onClick={() => setSection(item.id)}
            >
              {item.icon}<span>{item.label}</span>
            </button>
          ))}
        </nav>

        <section className="settings-content">
          {section === "capture" && <>
            <SettingHeading title="Capture" note="Replay clips are always one action away; full recordings are your choice." />
            <SettingCard title="Replay clips">
              <Toggle
                label="Keep replay buffer ready automatically"
                value={draft.replay.autoStart}
                onChange={(value) => update("replay", { autoStart: value, enabled: true })}
              />
              <div className="field-grid two">
                <Field label="Before hotkey">
                  <NumberInput value={draft.replay.preRollSeconds} suffix="sec" onChange={(value) => update("replay", { preRollSeconds: value })} />
                </Field>
                <Field label="After hotkey">
                  <NumberInput value={draft.replay.postRollSeconds} suffix="sec" onChange={(value) => update("replay", { postRollSeconds: value })} />
                </Field>
              </div>
            </SettingCard>
            <SettingCard title="Full recordings">
              <Toggle
                label="Automatically record detected games"
                value={draft.recording.autoRecordGames}
                onChange={(value) => update("recording", { autoRecordGames: value })}
              />
              <Field label="Quality">
                <div className="segmented">
                  {(["performance", "balanced", "quality", "custom"] as const).map((quality) => (
                    <button
                      className={draft.recording.quality === quality ? "active" : ""}
                      onClick={() => selectQuality(quality)}
                      key={quality}
                    >
                      {quality[0].toUpperCase() + quality.slice(1)}
                    </button>
                  ))}
                </div>
              </Field>
              <p className="preset-summary">
                {draft.recording.width} × {draft.recording.height} · {draft.recording.fps} FPS · {(draft.recording.bitrateKbps / 1000).toFixed(0)} Mbps
              </p>
            </SettingCard>
          </>}

          {section === "audio" && <>
            <SettingHeading title="Audio" note="Application audio avoids recording every system sound by default." />
            <SettingCard title="Sources">
              <Toggle
                label="Capture game / application audio"
                value={draft.audio.applicationAudio}
                onChange={(value) => update("audio", { applicationAudio: value })}
              />
              <Toggle
                label="Capture microphone"
                value={draft.audio.microphone}
                onChange={(value) => update("audio", { microphone: value })}
              />
              <Field label="Microphone device">
                <select
                  disabled={!draft.audio.microphone}
                  value={currentMicrophone}
                  onChange={(event) => update("audio", { microphoneDeviceId: event.target.value })}
                >
                  <option value="default">System default microphone</option>
                  {currentMicrophone !== "default" && !microphones.some((item) => item.value === currentMicrophone) && (
                    <option value={currentMicrophone}>{currentMicrophone}</option>
                  )}
                  {microphones
                    .filter((item) => item.value !== "default")
                    .map((item) => <option key={item.value} value={item.value} disabled={item.disabled}>{item.label}</option>)}
                </select>
              </Field>
            </SettingCard>
            <div className="settings-note">Audio is mixed into the normal playback track today. The Rust routing keeps game/application and microphone sources separate internally so multi-track output can be added without changing the capture UX.</div>
          </>}

          {section === "hotkeys" && <>
            <SettingHeading title="Hotkeys" note="Global shortcuts work while Clipture is in the tray." />
            <SettingCard title="Global shortcuts">
              <Field label="Save clip"><input value={draft.hotkeys.saveClip} onChange={(event) => update("hotkeys", { saveClip: event.target.value })} /></Field>
              <Field label="Start / stop full recording"><input value={draft.hotkeys.toggleRecording} onChange={(event) => update("hotkeys", { toggleRecording: event.target.value })} /></Field>
              <small className="field-help">Clipture validates and registers both shortcuts when you save. Conflicts are reported here instead of silently disabling the hotkey.</small>
            </SettingCard>
          </>}

          {section === "storage" && <>
            <SettingHeading title="Storage" note="Clips are never removed automatically." />
            <SettingCard title="Library">
              <Field label="Capture folder"><div className="path-input"><input value={draft.storage.root} onChange={(event) => update("storage", { root: event.target.value })} /><button type="button" onClick={async () => { const folder = await onChooseFolder(draft.storage.root); if (folder) update("storage", { root: folder }); }}>Browse</button></div></Field>
              <Field label="Full recording budget">
                <NumberInput value={draft.storage.recordingBudgetGb ?? 100} suffix="GB" onChange={(value) => update("storage", { recordingBudgetGb: value })} />
                <small className="field-help">When the budget is exceeded, the oldest unprotected full recordings are cleaned first.</small>
              </Field>
            </SettingCard>
          </>}

          {section === "behavior" && <>
            <SettingHeading title="Behavior" note="Designed to disappear into the background while you play." />
            <SettingCard title="Desktop">
              <Toggle label="Start Clipture with the system" value={draft.behavior.startWithSystem} onChange={(value) => update("behavior", { startWithSystem: value })} />
              <Toggle
                label="Keep capture running when the window closes"
                value={draft.behavior.closeToTray}
                onChange={(value) => update("behavior", { closeToTray: value, closeBehaviorConfirmed: true })}
              />
            </SettingCard>
            <SettingCard title="Clip feedback">
              <Toggle label="Desktop notification" value={draft.notifications.desktop} onChange={(value) => update("notifications", { desktop: value })} />
              <Toggle label="Confirmation sound" value={draft.notifications.sound} onChange={(value) => update("notifications", { sound: value })} />
              <Toggle label="In-app confirmation" value={draft.notifications.inApp} onChange={(value) => update("notifications", { inApp: value })} />
            </SettingCard>
          </>}

          {section === "advanced" && <>
            <SettingHeading title="Advanced" note="Normally Clipture lets libobs-rs choose the safest available path." />
            <SettingCard title="Capture engine">
              <Field label="Automatic capture method">
                <select
                  value={draft.advanced.captureMethod}
                  onChange={(event) => update("advanced", { captureMethod: event.target.value as AppSettings["advanced"]["captureMethod"] })}
                >
                  <option value="auto">Game capture when available</option>
                  <option value="game">Force game capture</option>
                  <option value="window">Use window capture for detected games</option>
                  <option value="monitor">Use the primary monitor</option>
                </select>
              </Field>
            </SettingCard>
            <SettingCard title="Custom output">
              <div className="field-grid two">
                <Field label="Width"><NumberInput value={draft.recording.width} onChange={(value) => updateRecording({ width: value })} /></Field>
                <Field label="Height"><NumberInput value={draft.recording.height} onChange={(value) => updateRecording({ height: value })} /></Field>
                <Field label="FPS"><NumberInput value={draft.recording.fps} onChange={(value) => updateRecording({ fps: value })} /></Field>
                <Field label="Video bitrate"><NumberInput value={draft.recording.bitrateKbps} suffix="Kbps" onChange={(value) => updateRecording({ bitrateKbps: value })} /></Field>
              </div>
              <small className="field-help">Changing resolution or FPS while a full recording is active is blocked so a settings save can never silently truncate the recording.</small>
            </SettingCard>
            <RuntimeCapabilities capabilities={capabilities} selectedEncoder={draft.advanced.encoder} onSelectEncoder={(encoder) => update("advanced", { encoder })} />
          </>}
        </section>
      </div>
    </main>
  );
}

function RuntimeCapabilities({
  capabilities,
  selectedEncoder,
  onSelectEncoder,
}: {
  capabilities?: ObsCapabilitiesSnapshot;
  selectedEncoder?: string;
  onSelectEncoder: (encoder?: string) => void;
}) {
  if (!capabilities) return <div className="settings-note">Reading the loaded OBS runtime and plugin capabilities…</div>;
  const h264Encoders = capabilities.videoEncoders.filter((encoder) => encoder.codec === "h264");
  return <>
    <SettingCard title="Video encoder">
      <Field label="Encoder used by replay clips and full recordings">
        <select value={selectedEncoder ?? ""} onChange={(event) => onSelectEncoder(event.target.value || undefined)}>
          <option value="">Automatic · prefer hardware</option>
          {h264Encoders.map((encoder) => <option key={encoder.id} value={encoder.id}>{encoder.label}{encoder.hardware ? " · hardware" : ""}</option>)}
        </select>
      </Field>
      <small className="field-help">Automatic is recommended. A manual choice is applied to both outputs; if a saved encoder disappears, Clipture falls back to automatic selection.</small>
    </SettingCard>
    <SettingCard title="Detected video encoders">
      <div className="encoder-list">
        {capabilities.videoEncoders.map((encoder) => (
          <div className="encoder-row" key={encoder.id}>
            <div><strong>{encoder.label}</strong><small>{encoder.id} · {encoder.codec ?? "unknown codec"}</small></div>
            <span className={encoder.hardware ? "hardware" : "software"}>{encoder.hardware ? "Hardware" : "Software"}</span>
          </div>
        ))}
        {!capabilities.videoEncoders.length && <p className="empty-inline">No video encoder was reported by the loaded OBS runtime.</p>}
      </div>
    </SettingCard>
    <details className="expert-browser">
      <summary>Expert · OBS property schemas <span>{capabilities.captureSources.length + capabilities.videoEncoders.length} runtime types</span></summary>
      <p>These controls are generated from the currently loaded OBS plugins, not hard-coded by Clipture. This view is intentionally diagnostic until a property is promoted into a supported Clipture setting.</p>
      {[...capabilities.captureSources, ...capabilities.videoEncoders].map((item) => (
        <details className="schema-group" key={item.id}>
          <summary>{item.label}<code>{item.id}</code></summary>
          <div className="schema-properties">
            {item.properties.filter((property) => property.visible).map((property) => <PropertyRow property={property} key={property.name} />)}
            {!item.properties.some((property) => property.visible) && <span className="empty-inline">No visible properties reported.</span>}
          </div>
        </details>
      ))}
    </details>
  </>;
}

function PropertyRow({ property }: { property: ObsPropertySchema }) {
  return <div className={`schema-property ${property.enabled ? "" : "disabled"}`}>
    <div><strong>{property.label}</strong><code>{property.name}</code></div>
    <span>{property.kind}{property.options.length ? ` · ${property.options.length} options` : ""}</span>
    {property.description && <small>{property.description}</small>}
  </div>;
}

function SettingHeading({ title, note }: { title: string; note: string }) {
  return <div className="settings-heading"><h2>{title}</h2><p>{note}</p></div>;
}
function SettingCard({ title, children }: { title: string; children: React.ReactNode }) {
  return <div className="setting-card"><h3>{title}</h3>{children}</div>;
}
function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="field"><span>{label}</span>{children}</label>;
}
function NumberInput({ value, onChange, suffix }: { value: number; onChange: (value: number) => void; suffix?: string }) {
  return <div className="number-input"><input type="number" value={value} onChange={(event) => onChange(Number(event.target.value))} />{suffix && <span>{suffix}</span>}</div>;
}
function Toggle({ label, value, onChange }: { label: string; value: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle-row"><span>{label}</span><input type="checkbox" checked={value} onChange={(event) => onChange(event.target.checked)} /><i /></label>;
}
