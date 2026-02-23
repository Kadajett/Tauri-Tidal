import { Outlet } from "react-router";
import { useCallback, useEffect, useRef } from "react";
import { Sidebar } from "./Sidebar";
import { FooterPlayer } from "@/components/player/FooterPlayer";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";
import { useLibrary } from "@/hooks/useLibrary";
import { useAuth } from "@/hooks/useAuth";
import { useAuthStore } from "@/stores/authStore";
import * as tauri from "@/lib/tauri";
import type { ProgressPayload } from "@/types/events";
import type { TrackChangedPayload } from "@/types/events";
import type { StateChangedPayload } from "@/types/events";

export function AppLayout() {
  useKeyboardShortcuts();
  const { loadPlaylists, loadFavorites } = useLibrary();
  const { checkAuth, handleCallback } = useAuth();

  const setProgress = usePlayerStore((s) => s.setProgress);
  const setState = usePlayerStore((s) => s.setState);
  const setCurrentTrack = usePlayerStore((s) => s.setCurrentTrack);
  const setQueue = useQueueStore((s) => s.setQueue);
  const setRepeatMode = useQueueStore((s) => s.setRepeatMode);
  const setShuffled = useQueueStore((s) => s.setShuffled);

  const handleProgress = useCallback(
    (payload: ProgressPayload) => {
      setProgress(payload.position, payload.duration);
    },
    [setProgress],
  );

  const handleTrackChanged = useCallback(
    (payload: TrackChangedPayload) => {
      setCurrentTrack({
        id: payload.track_id,
        title: payload.title,
        artistName: payload.artist,
        albumName: payload.album,
        duration: payload.duration,
        artworkUrl: payload.artwork_url,
        mediaTags: [],
      });
    },
    [setCurrentTrack],
  );

  const handleStateChanged = useCallback(
    (payload: StateChangedPayload) => {
      setState(payload.state);
    },
    [setState],
  );

  const syncQueue = useCallback(async () => {
    try {
      const queue = await tauri.getQueue();
      setQueue(queue.tracks, queue.currentIndex);
      setRepeatMode(queue.repeatMode);
      setShuffled(queue.shuffled);
    } catch (err) {
      console.error("Failed to sync queue:", err);
    }
  }, [setQueue, setRepeatMode, setShuffled]);

  // Set up event listeners
  useEffect(() => {
    const unlisteners = Promise.all([
      tauri.onProgress(handleProgress),
      tauri.onTrackChanged(handleTrackChanged),
      tauri.onStateChanged(handleStateChanged),
      tauri.onQueueChanged(syncQueue),
      tauri.onTrackEnded(syncQueue),
    ]);

    return () => {
      unlisteners.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [handleProgress, handleTrackChanged, handleStateChanged, syncQueue]);

  const authenticated = useAuthStore((s) => s.authenticated);

  // Check auth on mount
  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  // Only load user-specific data (playlists, favorites) when authenticated
  useEffect(() => {
    if (authenticated) {
      loadPlaylists();
      loadFavorites();
    }
  }, [authenticated, loadPlaylists, loadFavorites]);

  // Restore persisted player preferences and queue on mount
  useEffect(() => {
    tauri.getPlayerPrefs().then((prefs) => {
      if (prefs.muted) {
        usePlayerStore.getState().setVolume(0);
        // Manually set muted state with the real volume as previousVolume
        usePlayerStore.setState({ muted: true, previousVolume: prefs.volume });
      } else {
        usePlayerStore.getState().setVolume(prefs.volume);
      }
    }).catch((err) => console.error("Failed to load player prefs:", err));

    tauri.loadSavedQueue().then((queue) => {
      setQueue(queue.tracks, queue.currentIndex);
      setRepeatMode(queue.repeatMode);
      setShuffled(queue.shuffled);
      // Set the current track in the player store (without starting playback)
      if (queue.currentIndex != null && queue.tracks[queue.currentIndex]) {
        const track = queue.tracks[queue.currentIndex];
        usePlayerStore.getState().setCurrentTrack(track);
      }
    }).catch((err) => console.error("Failed to load saved queue:", err));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Persist volume/muted changes with debounce
  const savePrefsTimer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => {
    const unsub = usePlayerStore.subscribe(
      (state, prev) => {
        if (state.volume !== prev.volume || state.muted !== prev.muted) {
          clearTimeout(savePrefsTimer.current);
          savePrefsTimer.current = setTimeout(() => {
            const { volume, muted, previousVolume } = usePlayerStore.getState();
            const realVolume = muted ? previousVolume : volume;
            tauri.savePlayerPrefs(realVolume, muted).catch((err) =>
              console.error("Failed to save player prefs:", err),
            );
          }, 500);
        }
      },
    );
    return () => {
      unsub();
      clearTimeout(savePrefsTimer.current);
    };
  }, []);

  // Persist queue changes with debounce
  const saveQueueTimer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => {
    const unsub = useQueueStore.subscribe(
      (state, prev) => {
        if (state.tracks !== prev.tracks || state.currentIndex !== prev.currentIndex ||
            state.repeatMode !== prev.repeatMode || state.shuffled !== prev.shuffled) {
          clearTimeout(saveQueueTimer.current);
          saveQueueTimer.current = setTimeout(() => {
            tauri.saveQueueState().catch((err) =>
              console.error("Failed to save queue state:", err),
            );
          }, 1000);
        }
      },
    );
    return () => {
      unsub();
      clearTimeout(saveQueueTimer.current);
    };
  }, []);

  // Listen for deep link auth callbacks
  useEffect(() => {
    let cancelled = false;
    import("@tauri-apps/plugin-deep-link").then(({ onOpenUrl }) => {
      if (cancelled) return;
      onOpenUrl((urls) => {
        for (const url of urls) {
          try {
            const parsed = new URL(url);
            if (parsed.pathname === "/auth/callback" || parsed.host === "auth") {
              const code = parsed.searchParams.get("code");
              if (code) {
                handleCallback(code);
              }
            }
          } catch {
            // Not a valid URL, ignore
          }
        }
      });
    });
    return () => {
      cancelled = true;
    };
  }, [handleCallback]);

  return (
    <div className="flex h-dvh flex-col">
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="flex-1 overflow-auto">
          <Outlet />
        </main>
      </div>
      <FooterPlayer />
    </div>
  );
}
