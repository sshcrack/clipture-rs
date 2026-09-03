import { useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Check, Scissors, X } from "lucide-react";
import { desktop } from "../lib/desktop";
import type { LibraryItem } from "../lib/types";

export function QuickEditor({ item, onClose, onSaved }: { item: LibraryItem; onClose: () => void; onSaved: () => void }) {
  const initialDuration = Math.max(0.1, (item.durationMs ?? 40_000) / 1000);
  const [duration, setDuration] = useState(initialDuration);
  const [start, setStart] = useState(0);
  const [end, setEnd] = useState(initialDuration);
  const [title, setTitle] = useState(item.title);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const source = useMemo(() => {
    try { return convertFileSrc(item.path); } catch { return ""; }
  }, [item.path]);

  const save = async () => {
    setBusy(true);
    setError(undefined);
    try {
      await desktop.trim({ id: item.id, startSeconds: start, endSeconds: end, title: title.trim() || item.title });
      onSaved();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  return <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
    <div className="editor-modal">
      <header>
        <div><span className="modal-icon"><Scissors size={17} /></span><div><small>Quick edit</small><strong>{item.title}</strong></div></div>
        <button onClick={onClose} aria-label="Close editor"><X size={18} /></button>
      </header>
      <div className="editor-preview">
        {source ? <video
          src={source}
          controls
          preload="metadata"
          onLoadedMetadata={(event) => {
            const actualDuration = event.currentTarget.duration;
            if (Number.isFinite(actualDuration) && actualDuration > 0) {
              setDuration(actualDuration);
              setEnd(actualDuration);
            }
          }}
        /> : <div className="preview-fallback"><Scissors size={24} /><span>Video preview is available in the desktop app.</span></div>}
      </div>
      <div className="trim-panel">
        <label className="field"><span>New clip title</span><input value={title} maxLength={120} onChange={(event) => setTitle(event.target.value)} /></label>
        <div className="trim-meta"><span>Start <strong>{start.toFixed(1)}s</strong></span><span>New length <strong>{Math.max(0, end - start).toFixed(1)}s</strong></span><span>End <strong>{end.toFixed(1)}s</strong></span></div>
        <div className="dual-range">
          <div className="trim-selection" style={{ left: `${start / duration * 100}%`, right: `${100 - end / duration * 100}%` }} />
          <input type="range" min="0" max={duration} step="0.1" value={start} onChange={(event) => setStart(Math.min(Number(event.target.value), end - 0.1))} />
          <input type="range" min="0" max={duration} step="0.1" value={end} onChange={(event) => setEnd(Math.max(Number(event.target.value), start + 0.1))} />
        </div>
        {error && <div className="editor-error">{error}</div>}
      </div>
      <footer><span>Original file stays untouched.</span><div><button className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" onClick={save} disabled={busy || end <= start}><Check size={16} />{busy ? "Saving…" : "Save as new clip"}</button></div></footer>
    </div>
  </div>;
}
