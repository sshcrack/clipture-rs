import { useMemo, useState } from "react";
import { Film, FolderOpen, Search, Shield, SlidersHorizontal, Trash2 } from "lucide-react";
import { QuickEditor } from "../components/QuickEditor";
import type { LibraryItem } from "../lib/types";

type Filter = "all" | "clip" | "recording";
const size = (bytes: number) => bytes > 1_000_000_000 ? `${(bytes / 1_000_000_000).toFixed(1)} GB` : `${Math.round(bytes / 1_000_000)} MB`;
const time = (ms?: number) => ms ? `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, "0")}` : "—";

export function LibraryPage({ items, onReveal, onDelete, onProtect, onRefresh }: {
  items: LibraryItem[];
  onReveal: (id: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onProtect: (id: string, value: boolean) => Promise<void>;
  onRefresh: () => Promise<void> | void;
}) {
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<LibraryItem>();
  const visible = useMemo(
    () => items.filter((item) => (filter === "all" || item.kind === filter) && `${item.title} ${item.game}`.toLowerCase().includes(query.toLowerCase())),
    [items, filter, query],
  );

  return <main className="page library-page">
    <header className="page-header"><div><p className="eyebrow">Local library</p><h1>Everything you kept.</h1><p className="header-note">Clips and full recordings stay ordinary files on your disk.</p></div></header>
    <div className="library-toolbar">
      <div className="filter-group">{(["all", "clip", "recording"] as Filter[]).map((value) => <button className={filter === value ? "active" : ""} key={value} onClick={() => setFilter(value)}>{value === "all" ? "All" : value === "clip" ? "Clips" : "Recordings"}</button>)}</div>
      <label className="search-box"><Search size={16} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search captures" /></label>
    </div>
    <div className="library-table-wrap">
      <div className="library-table-head"><span>Capture</span><span>Type</span><span>Duration</span><span>Size</span><span>Saved</span><span /></div>
      <div className="library-table">
        {visible.map((item) => <div className="library-row" key={item.id}>
          <button className="library-title" onClick={() => setEditing(item)}><span className={`row-thumb ${item.kind}`}>{item.kind === "clip" ? <SlidersHorizontal size={17} /> : <Film size={17} />}</span><span><strong>{item.title}</strong><small>{item.game}</small></span></button>
          <span className="type-chip">{item.kind}</span><span>{time(item.durationMs)}</span><span>{size(item.sizeBytes)}</span><span>{new Date(item.createdAt).toLocaleDateString(undefined, { month: "short", day: "numeric" })}</span>
          <div className="row-actions">
            {item.kind === "recording" && <button title={item.protected ? "Allow automatic cleanup" : "Protect from cleanup"} className={item.protected ? "protected" : ""} onClick={async () => { await onProtect(item.id, !item.protected); await onRefresh(); }}><Shield size={16} /></button>}
            <button title="Show in folder" onClick={() => onReveal(item.id)}><FolderOpen size={16} /></button>
            <button title="Delete" onClick={async () => { if (window.confirm(`Delete “${item.title}” from disk?`)) await onDelete(item.id); }}><Trash2 size={16} /></button>
          </div>
        </div>)}
        {!visible.length && <div className="empty-library"><Film size={24} /><strong>No captures here</strong><span>Change the filter or save a clip with your capture hotkey.</span></div>}
      </div>
    </div>
    {editing && <QuickEditor item={editing} onClose={() => setEditing(undefined)} onSaved={() => { setEditing(undefined); void onRefresh(); }} />}
  </main>;
}
