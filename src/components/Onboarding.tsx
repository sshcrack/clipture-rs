import { useState } from "react";
import { ArrowRight, Check, FolderOpen, Keyboard, Mic2, Sparkles } from "lucide-react";
import type { AppSettings, ObsCapabilitiesSnapshot } from "../lib/types";

export function Onboarding({
  settings,
  capabilities,
  onChooseFolder,
  onComplete,
}: {
  settings: AppSettings;
  capabilities?: ObsCapabilitiesSnapshot;
  onChooseFolder: (defaultPath?: string) => Promise<string | undefined>;
  onComplete: (settings: AppSettings) => Promise<void>;
}) {
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState(settings);
  const [finishing, setFinishing] = useState(false);
  const [error, setError] = useState<string>();

  const update = (key: keyof AppSettings, patch: Record<string, unknown>) =>
    setDraft((current) => ({ ...current, [key]: { ...(current[key] as object), ...patch } }) as AppSettings);

  const finish = async () => {
    setFinishing(true);
    setError(undefined);
    try {
      await onComplete({ ...draft, onboardingComplete: true });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setFinishing(false);
    }
  };

  const microphones = capabilities?.microphones ?? [];
  const microphone = draft.audio.microphoneDeviceId ?? "default";
  const steps = [
    <div className="onboarding-step" key="welcome">
      <div className="onboarding-hero"><Sparkles size={25} /></div>
      <p className="eyebrow">Welcome to Clipture</p>
      <h1>Never think about recording before the moment happens.</h1>
      <p>Clipture keeps a private rolling buffer ready in the background. Press one key and the moment around it becomes a clip.</p>
      <div className="onboarding-window"><span>−{draft.replay.preRollSeconds}s</span><i /><b>{draft.hotkeys.saveClip}</b><i /><span>+{draft.replay.postRollSeconds}s</span></div>
    </div>,
    <div className="onboarding-step" key="clip">
      <div className="onboarding-icon"><Keyboard size={20} /></div>
      <p className="eyebrow">Your clip</p>
      <h2>Choose the moment around your hotkey.</h2>
      <div className="field-grid two">
        <label className="field"><span>Before</span><div className="number-input"><input type="number" value={draft.replay.preRollSeconds} onChange={(event) => update("replay", { preRollSeconds: Number(event.target.value) })} /><span>sec</span></div></label>
        <label className="field"><span>After</span><div className="number-input"><input type="number" value={draft.replay.postRollSeconds} onChange={(event) => update("replay", { postRollSeconds: Number(event.target.value) })} /><span>sec</span></div></label>
      </div>
      <label className="field"><span>Save clip hotkey</span><input value={draft.hotkeys.saveClip} onChange={(event) => update("hotkeys", { saveClip: event.target.value })} /></label>
    </div>,
    <div className="onboarding-step" key="audio">
      <div className="onboarding-icon"><Mic2 size={20} /></div>
      <p className="eyebrow">Audio</p>
      <h2>Capture what matters, not your whole desktop.</h2>
      <label className="toggle-row"><span>Game / application audio</span><input type="checkbox" checked={draft.audio.applicationAudio} onChange={(event) => update("audio", { applicationAudio: event.target.checked })} /><i /></label>
      <label className="toggle-row"><span>Microphone</span><input type="checkbox" checked={draft.audio.microphone} onChange={(event) => update("audio", { microphone: event.target.checked })} /><i /></label>
      <label className="field"><span>Microphone device</span><select disabled={!draft.audio.microphone} value={microphone} onChange={(event) => update("audio", { microphoneDeviceId: event.target.value })}>
        <option value="default">System default microphone</option>
        {microphone !== "default" && !microphones.some((item) => item.value === microphone) && <option value={microphone}>{microphone}</option>}
        {microphones.filter((item) => item.value !== "default").map((item) => <option key={item.value} value={item.value} disabled={item.disabled}>{item.label}</option>)}
      </select></label>
    </div>,
    <div className="onboarding-step" key="storage">
      <div className="onboarding-icon"><FolderOpen size={20} /></div>
      <p className="eyebrow">Storage & background</p>
      <h2>Keep your captures where you expect them.</h2>
      <label className="field"><span>Capture folder</span><div className="path-input"><input value={draft.storage.root} onChange={(event) => update("storage", { root: event.target.value })} /><button type="button" onClick={async () => { const folder = await onChooseFolder(draft.storage.root); if (folder) update("storage", { root: folder }); }}>Browse</button></div></label>
      <label className="toggle-row"><span>Keep capture running when I close the window</span><input type="checkbox" checked={draft.behavior.closeToTray} onChange={(event) => update("behavior", { closeToTray: event.target.checked })} /><i /></label>
      <label className="toggle-row"><span>Start Clipture with the system</span><input type="checkbox" checked={draft.behavior.startWithSystem} onChange={(event) => update("behavior", { startWithSystem: event.target.checked })} /><i /></label>
      <p className="onboarding-small">Files are organized by game and date. Clips are never automatically deleted. The first time you close Clipture, it will confirm whether you want it to stay in the tray.</p>
    </div>,
  ];

  return <div className="onboarding-backdrop"><div className="onboarding-card">
    {steps[step]}
    {error && <div className="onboarding-error">{error}</div>}
    <footer>
      <div className="onboarding-footer-copy"><span>{step + 1} / {steps.length}</span><button className="text-button" disabled={finishing} onClick={finish}>Skip setup</button></div>
      <button className="primary-button" disabled={finishing} onClick={async () => { if (step < steps.length - 1) setStep(step + 1); else await finish(); }}>
        {step === steps.length - 1 ? <><Check size={16} />{finishing ? "Starting…" : "Start Clipture"}</> : <>Continue<ArrowRight size={16} /></>}
      </button>
    </footer>
  </div></div>;
}
