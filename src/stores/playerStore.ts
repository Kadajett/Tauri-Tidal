import { create } from "zustand";
import type { PlaybackState } from "@/types/player";
import type { Track } from "@/types/track";

interface PlayerState {
  state: PlaybackState;
  currentTrack: Track | null;
  position: number;
  duration: number;
  volume: number;
  muted: boolean;
  previousVolume: number;
  codec: string | null;
  quality: string | null;
  expanded: boolean;

  setState: (state: PlaybackState) => void;
  setCurrentTrack: (track: Track | null) => void;
  setCodecInfo: (codec: string | null, quality: string | null) => void;
  setPosition: (position: number) => void;
  setDuration: (duration: number) => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  setProgress: (position: number, duration: number) => void;
  setExpanded: (expanded: boolean) => void;
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  state: "stopped",
  currentTrack: null,
  position: 0,
  duration: 0,
  volume: 1.0,
  muted: false,
  previousVolume: 1.0,
  codec: null,
  quality: null,
  expanded: false,

  setState: (state) => set({ state }),
  setCurrentTrack: (track) => set({ currentTrack: track }),
  setCodecInfo: (codec, quality) => set({ codec, quality }),
  setPosition: (position) => set({ position }),
  setDuration: (duration) => set({ duration }),
  setVolume: (volume) => set({ volume, muted: false }),
  toggleMute: () => {
    const { muted, volume, previousVolume } = get();
    if (muted) {
      set({ muted: false, volume: previousVolume });
    } else {
      set({ muted: true, previousVolume: volume, volume: 0 });
    }
  },
  setProgress: (position, duration) => set({ position, duration }),
  setExpanded: (expanded) => set({ expanded }),
}));
