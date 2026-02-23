import { useCallback } from "react";
import { usePlayerStore } from "@/stores/playerStore";
import * as tauri from "@/lib/tauri";
import type { Track } from "@/types/track";

export function usePlayback() {
  const state = usePlayerStore((s) => s.state);

  const play = useCallback(async (trackId: string) => {
    try {
      await tauri.playTrack(trackId);
    } catch (err) {
      console.error("Play failed:", err);
    }
  }, []);

  const playTracks = useCallback(
    async (tracks: Track[], startIndex: number) => {
      try {
        await tauri.playTracks(tracks, startIndex);
      } catch (err) {
        console.error("Play tracks failed:", err);
      }
    },
    [],
  );

  const togglePlayPause = useCallback(async () => {
    try {
      if (state === "playing") {
        await tauri.pausePlayback();
      } else {
        await tauri.resumePlayback();
      }
    } catch (err) {
      console.error("Toggle play/pause failed:", err);
    }
  }, [state]);

  const seek = useCallback(async (seconds: number) => {
    try {
      await tauri.seekTo(seconds);
    } catch (err) {
      console.error("Seek failed:", err);
    }
  }, []);

  const nextTrack = useCallback(async () => {
    try {
      await tauri.nextTrack();
    } catch (err) {
      console.error("Next track failed:", err);
    }
  }, []);

  const previousTrack = useCallback(async () => {
    try {
      await tauri.previousTrack();
    } catch (err) {
      console.error("Previous track failed:", err);
    }
  }, []);

  const setVolume = useCallback(async (vol: number) => {
    try {
      await tauri.setVolume(vol);
      usePlayerStore.getState().setVolume(vol);
    } catch (err) {
      console.error("Set volume failed:", err);
    }
  }, []);

  const toggleMute = useCallback(async () => {
    const { muted, previousVolume } = usePlayerStore.getState();
    usePlayerStore.getState().toggleMute();
    try {
      if (muted) {
        // Unmuting: restore previous volume
        await tauri.setVolume(previousVolume);
      } else {
        // Muting: set volume to 0
        await tauri.setVolume(0);
      }
    } catch (err) {
      console.error("Toggle mute failed:", err);
    }
  }, []);

  return {
    play,
    playTracks,
    togglePlayPause,
    seek,
    nextTrack,
    previousTrack,
    setVolume,
    toggleMute,
    isPlaying: state === "playing",
  };
}
