import { useEffect, useRef, useState, type ReactNode } from "react";
import { desktop } from "../lib/desktop";

interface PreviewRect {
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
}

function fitAspectRatio(rect: DOMRect, aspectRatio: number): PreviewRect {
  let width = rect.width;
  let height = width / aspectRatio;
  if (height > rect.height) {
    height = rect.height;
    width = height * aspectRatio;
  }
  return {
    x: rect.left + (rect.width - width) / 2,
    y: rect.top + (rect.height - height) / 2,
    width,
    height,
    visible: true,
  };
}

export function ObsPreview({
  active,
  aspectRatio,
  children,
}: {
  active: boolean;
  aspectRatio: number;
  children: ReactNode;
}) {
  const slotRef = useRef<HTMLDivElement>(null);
  const lastSent = useRef("");
  const frame = useRef<number | undefined>(undefined);
  const mounted = useRef(true);
  const generation = useRef(0);
  const [nativeActive, setNativeActive] = useState(false);

  useEffect(() => {
    const effectGeneration = ++generation.current;
    mounted.current = true;
    const slot = slotRef.current;
    if (!slot) return;

    const send = async () => {
      frame.current = undefined;
      const rect = slot.getBoundingClientRect();
      const fullyVisible = rect.width > 4
        && rect.height > 4
        && rect.left >= 0
        && rect.top >= 0
        && rect.right <= window.innerWidth
        && rect.bottom <= window.innerHeight
        && document.visibilityState === "visible";
      const fitted = fitAspectRatio(rect, Math.max(0.1, aspectRatio));
      const payload = {
        ...fitted,
        visible: active && fullyVisible,
      };
      const key = [
        Math.round(payload.x),
        Math.round(payload.y),
        Math.round(payload.width),
        Math.round(payload.height),
        payload.visible,
      ].join(":");
      if (key === lastSent.current) return;
      lastSent.current = key;
      try {
        const supported = await desktop.updatePreview(payload);
        if (mounted.current && generation.current === effectGeneration) {
          setNativeActive(supported && payload.visible);
        }
      } catch {
        if (mounted.current && generation.current === effectGeneration) setNativeActive(false);
      }
    };

    const schedule = () => {
      if (frame.current !== undefined) return;
      frame.current = window.requestAnimationFrame(() => void send());
    };

    const observer = new ResizeObserver(schedule);
    observer.observe(slot);
    const scroller = document.querySelector(".content-shell");
    scroller?.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule, { passive: true });
    document.addEventListener("visibilitychange", schedule);
    schedule();

    return () => {
      mounted.current = false;
      generation.current += 1;
      observer.disconnect();
      scroller?.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      document.removeEventListener("visibilitychange", schedule);
      if (frame.current !== undefined) window.cancelAnimationFrame(frame.current);
      lastSent.current = "";
      void desktop.updatePreview({ x: 0, y: 0, width: 0, height: 0, visible: false }).catch(() => undefined);
    };
  }, [active, aspectRatio]);

  return <div ref={slotRef} className={`obs-preview-slot ${nativeActive ? "native-preview-active" : ""}`}>
    {children}
  </div>;
}
