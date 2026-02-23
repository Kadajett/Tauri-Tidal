import { useCallback, useEffect, useRef, useState } from "react";
import { usePlayerStore } from "@/stores/playerStore";

export function useAnimatedProgress() {
  const position = usePlayerStore((s) => s.position);
  const duration = usePlayerStore((s) => s.duration);
  const state = usePlayerStore((s) => s.state);

  const [displayPosition, setDisplayPosition] = useState(0);
  const lastSyncRef = useRef({ position: 0, timestamp: performance.now() });
  const rafRef = useRef<number>(0);
  const draggingRef = useRef(false);

  // Sync from backend
  useEffect(() => {
    lastSyncRef.current = {
      position,
      timestamp: performance.now(),
    };
    if (!draggingRef.current) {
      setDisplayPosition(position);
    }
  }, [position]);

  // Animation loop
  useEffect(() => {
    if (state !== "playing" || draggingRef.current) {
      cancelAnimationFrame(rafRef.current);
      return;
    }

    const tick = () => {
      const now = performance.now();
      const elapsed = (now - lastSyncRef.current.timestamp) / 1000;
      const interpolated = lastSyncRef.current.position + elapsed;
      setDisplayPosition(Math.min(interpolated, duration));
      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [state, duration]);

  const setDragging = useCallback((isDragging: boolean) => {
    draggingRef.current = isDragging;
  }, []);

  const fraction = duration > 0 ? displayPosition / duration : 0;

  return {
    displayPosition,
    setDisplayPosition,
    fraction,
    duration,
    setDragging,
  };
}
