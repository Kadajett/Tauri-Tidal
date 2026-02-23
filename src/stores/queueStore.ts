import { create } from "zustand";
import type { RepeatMode } from "@/types/player";
import type { Track } from "@/types/track";

interface QueueStoreState {
  tracks: Track[];
  currentIndex: number | null;
  repeatMode: RepeatMode;
  shuffled: boolean;

  setQueue: (tracks: Track[], currentIndex: number | null) => void;
  setCurrentIndex: (index: number | null) => void;
  setRepeatMode: (mode: RepeatMode) => void;
  setShuffled: (shuffled: boolean) => void;
}

export const useQueueStore = create<QueueStoreState>((set) => ({
  tracks: [],
  currentIndex: null,
  repeatMode: "off",
  shuffled: false,

  setQueue: (tracks, currentIndex) => set({ tracks, currentIndex }),
  setCurrentIndex: (currentIndex) => set({ currentIndex }),
  setRepeatMode: (repeatMode) => set({ repeatMode }),
  setShuffled: (shuffled) => set({ shuffled }),
}));
