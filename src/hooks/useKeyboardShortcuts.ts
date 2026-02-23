import { useEffect } from "react";
import { KEYBOARD_SHORTCUTS } from "@/lib/constants";
import { usePlayback } from "./usePlayback";
import { usePlayerStore } from "@/stores/playerStore";

export function useKeyboardShortcuts() {
  const { togglePlayPause, seek, nextTrack, previousTrack, setVolume, toggleMute } =
    usePlayback();
  const position = usePlayerStore((s) => s.position);
  const volume = usePlayerStore((s) => s.volume);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInput =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      if (isInput && e.key !== "Escape") return;

      switch (e.key) {
        case KEYBOARD_SHORTCUTS.TOGGLE_PLAY:
          e.preventDefault();
          togglePlayPause();
          break;
        case KEYBOARD_SHORTCUTS.SEEK_FORWARD:
          e.preventDefault();
          seek(position + 5);
          break;
        case KEYBOARD_SHORTCUTS.SEEK_BACK:
          e.preventDefault();
          seek(Math.max(0, position - 5));
          break;
        case KEYBOARD_SHORTCUTS.VOLUME_UP:
          e.preventDefault();
          setVolume(Math.min(1, volume + 0.05));
          break;
        case KEYBOARD_SHORTCUTS.VOLUME_DOWN:
          e.preventDefault();
          setVolume(Math.max(0, volume - 0.05));
          break;
        case KEYBOARD_SHORTCUTS.NEXT_TRACK:
          nextTrack();
          break;
        case KEYBOARD_SHORTCUTS.PREV_TRACK:
          previousTrack();
          break;
        case KEYBOARD_SHORTCUTS.TOGGLE_MUTE:
          toggleMute();
          break;
        case KEYBOARD_SHORTCUTS.FOCUS_SEARCH: {
          e.preventDefault();
          const searchInput = document.querySelector<HTMLInputElement>(
            '[data-search-input]',
          );
          searchInput?.focus();
          break;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    togglePlayPause,
    seek,
    nextTrack,
    previousTrack,
    setVolume,
    toggleMute,
    position,
    volume,
  ]);
}
