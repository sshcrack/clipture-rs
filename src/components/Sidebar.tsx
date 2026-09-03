import { Clapperboard, Home, Settings, Video } from "lucide-react";

export type Page = "home" | "library" | "settings";

export function Sidebar({ page, setPage, captureActive }: { page: Page; setPage: (page: Page) => void; captureActive: boolean }) {
  return <aside className="sidebar">
    <div className="brand" aria-label="Clipture">
      <div className="brand-mark"><Clapperboard size={18} strokeWidth={2.2} /></div>
      <span>clipture</span>
    </div>
    <nav className="primary-nav" aria-label="Main navigation">
      <NavButton active={page === "home"} onClick={() => setPage("home")} icon={<Home size={18} />} label="Home" />
      <NavButton active={page === "library"} onClick={() => setPage("library")} icon={<Video size={18} />} label="Library" />
      <NavButton active={page === "settings"} onClick={() => setPage("settings")} icon={<Settings size={18} />} label="Settings" />
    </nav>
    <div className="sidebar-spacer" />
    <div className="engine-pill">
      <span className={`engine-dot ${captureActive ? "active" : ""}`} />
      <div><strong>{captureActive ? "Capture ready" : "Capture idle"}</strong><span>OBS engine</span></div>
    </div>
  </aside>;
}

function NavButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return <button className={`nav-button ${active ? "active" : ""}`} onClick={onClick}>{icon}<span>{label}</span></button>;
}
