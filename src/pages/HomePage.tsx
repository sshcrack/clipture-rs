import { useEffect, useMemo, useState } from "react";
import {
  ChevronDown,
  CircleStop,
  Clock3,
  Film,
  Gamepad2,
  Mic2,
  MonitorUp,
  Play,
  Radio,
  Save,
  Sparkles,
} from "lucide-react";
import type { AppSettings, CaptureSnapshot, CaptureSource, LibraryItem } from "../lib/types";

function duration(ms: number) {
  const total = Math.max(0, Math.floor(ms / 1000));
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

function audioSummary(settings: AppSettings) {
  if (settings.audio.applicationAudio && settings.audio.microphone) return "App + microphone";
  if (settings.audio.applicationAudio) return "Application audio";
  if (settings.audio.microphone) return "Microphone only";
  return "Off";
}

export function HomePage({
  capture,
  settings,
  recent,
  sources,
  onSaveClip,
  onToggleReplay,
  onToggleRecording,
  onSelectSource,
  onOpenLibrary,
}: {
  capture: CaptureSnapshot;
  settings: AppSettings;
  recent: LibraryItem[];
  sources: CaptureSource[];
  onSaveClip: () => void;
  onToggleReplay: () => void;
  onToggleRecording: () => void;
  onSelectSource: (source: CaptureSource) => void;
  onOpenLibrary: () => void;
}) {
  const [now, setNow] = useState(Date.now());
  const [sourceOpen, setSourceOpen] = useState(false);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const recordingTime = capture.recordingActive && capture.recordingStartedAt
    ? duration(now - capture.recordingStartedAt)
    : "00:00";
  const pending = capture.pendingClips[0];
  const pendingSeconds = pending ? Math.max(0, Math.ceil((pending.savesAt - now) / 1000)) : 0;
  const sourceIcon = capture.source?.kind === "game" ? <Gamepad2 size={16} /> : <MonitorUp size={16} />;
  const windowSeconds = Math.max(1, settings.replay.preRollSeconds + settings.replay.postRollSeconds);
  const marker = useMemo(
    () => `${Math.round(settings.replay.preRollSeconds / windowSeconds * 100)}%`,
    [settings.replay.preRollSeconds, windowSeconds],
  );

  return <main className="page home-page">
    <header className="page-header">
      <div><p className="eyebrow">Capture desk</p><h1>Your moments, already rolling.</h1></div>
      <div className="source-picker-wrap">
        <button className="source-picker" onClick={() => setSourceOpen((value) => !value)} aria-expanded={sourceOpen}>
          <span className="source-icon">{sourceIcon}</span>
          <span><small>Capturing</small><strong>{capture.source?.label ?? "No source"}</strong></span>
          <ChevronDown size={15} />
        </button>
        {sourceOpen && <div className="source-menu">
          {sources.map((source) => <button key={source.id} onClick={() => { onSelectSource(source); setSourceOpen(false); }}>
            <span>{source.kind === "game" ? <Gamepad2 size={15} /> : source.kind === "auto" ? <Sparkles size={15} /> : <MonitorUp size={15} />}</span>
            <span><strong>{source.label}</strong><small>{source.subtitle}</small></span>
          </button>)}
        </div>}
      </div>
    </header>

    {capture.phase === "error" && <div className="error-banner"><strong>Capture needs attention</strong><span>{capture.message ?? "OBS could not start. Open Settings → Advanced for diagnostics."}</span></div>}

    <section className="capture-stage">
      <div className="stage-visual" aria-label="Capture status preview">
        <div className="stage-grid" />
        <div className="stage-topline">
          <span className={`live-badge ${capture.recordingActive ? "recording" : ""}`}><i />{capture.recordingActive ? `REC ${recordingTime}` : capture.replayActive ? "REPLAY READY" : "IDLE"}</span>
          <span>{capture.outputResolution} · {capture.fps} FPS</span>
        </div>
        <div className="stage-center">
          <div className="source-glyph">{sourceIcon}</div>
          <h2>{capture.source?.label ?? "Choose what to capture"}</h2>
          <p>{capture.source?.subtitle ?? "Pick a game, window, or monitor."}</p>
        </div>
        <div className="capture-rail">
          <div className="rail-labels"><span>−{settings.replay.preRollSeconds}s</span><span className="rail-now">hotkey</span><span>+{settings.replay.postRollSeconds}s</span></div>
          <div className="rail-track"><div className="rail-buffer" /><div className="rail-marker" style={{ left: marker }}><i /></div><div className="rail-post" style={{ left: marker }} /></div>
        </div>
      </div>

      <div className="stage-actions">
        <div className="action-status">
          <span className="status-kicker">Replay buffer</span>
          <strong>{capture.replayActive ? "Ready to clip" : "Paused"}</strong>
          <p>{capture.replayActive ? `${settings.replay.preRollSeconds}s before + ${settings.replay.postRollSeconds}s after every press.` : "Start the rolling buffer whenever you want clipping available."}</p>
        </div>
        {capture.replayActive ? (
          <button className="save-clip-button" onClick={onSaveClip}>
            <span className="button-icon"><Save size={20} /></span>
            <span><strong>{pending ? `Saving in ${pendingSeconds}s` : "Save clip"}</strong><small>{settings.hotkeys.saveClip}</small></span>
          </button>
        ) : (
          <button className="save-clip-button replay-start" onClick={onToggleReplay}>
            <span className="button-icon"><Play size={20} /></span>
            <span><strong>Start replay buffer</strong><small>Clipping stays local</small></span>
          </button>
        )}
        <button className={`record-button ${capture.recordingActive ? "stop" : ""}`} onClick={onToggleRecording}>
          {capture.recordingActive ? <CircleStop size={18} /> : <Radio size={18} />}
          <span>{capture.recordingActive ? "Stop full recording" : "Record full session"}</span>
        </button>
        <div className="capture-facts">
          <Fact icon={<Film size={15} />} label="Encoder" value={capture.encoder ?? "Auto select"} />
          <Fact icon={<Mic2 size={15} />} label="Audio" value={audioSummary(settings)} />
          <Fact icon={<Clock3 size={15} />} label="Recording" value={capture.recordingActive ? recordingTime : "Off"} />
        </div>
      </div>
    </section>

    <section className="recent-section">
      <div className="section-heading"><div><p className="eyebrow">Latest saves</p><h2>Recent captures</h2></div><button className="text-button" onClick={onOpenLibrary}>Open library <span>→</span></button></div>
      <div className="recent-list">
        {recent.slice(0, 4).map((item) => <button className="recent-item" key={item.id} onClick={onOpenLibrary}>
          <div className={`media-thumb ${item.kind}`}><span>{item.kind === "clip" ? <Save size={17} /> : <Film size={17} />}</span></div>
          <div className="recent-copy"><strong>{item.title}</strong><span>{item.kind === "clip" ? "Clip" : "Recording"} · {item.game}</span></div>
          <span className="recent-duration">{item.durationMs ? duration(item.durationMs) : "—"}</span>
        </button>)}
        {!recent.length && <div className="empty-recent"><strong>No captures yet</strong><span>Press {settings.hotkeys.saveClip} while the replay buffer is ready.</span></div>}
      </div>
    </section>
  </main>;
}

function Fact({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return <div className="capture-fact"><span>{icon}</span><div><small>{label}</small><strong>{value}</strong></div></div>;
}
