import { useCallback } from "react";
import { useLibraryStore } from "@/stores/libraryStore";
import * as tauri from "@/lib/tauri";

export function useLibrary() {
  const {
    setPlaylists,
    setFavorites,
    appendFavorites,
    addFavorite,
    removeFavorite,
    setLoading,
    setLoadingMore,
  } = useLibraryStore();

  const loadPlaylists = useCallback(async () => {
    setLoading(true);
    try {
      const playlists = await tauri.getPlaylists();
      setPlaylists(playlists);
    } catch (err) {
      console.error("Failed to load playlists:", err);
    } finally {
      setLoading(false);
    }
  }, [setPlaylists, setLoading]);

  const loadFavorites = useCallback(async () => {
    try {
      const page = await tauri.getFavorites();
      setFavorites(page.tracks, page.nextCursor ?? null, page.hasMore);
    } catch (err) {
      console.error("Failed to load favorites:", err);
    }
  }, [setFavorites]);

  const loadMoreFavorites = useCallback(async () => {
    const cursor = useLibraryStore.getState().favoritesNextCursor;
    if (!cursor) return;
    setLoadingMore(true);
    try {
      const page = await tauri.getFavorites(cursor);
      appendFavorites(page.tracks, page.nextCursor ?? null, page.hasMore);
    } catch (err) {
      console.error("Failed to load more favorites:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [appendFavorites, setLoadingMore]);

  const toggleFavorite = useCallback(
    async (trackId: string, isFavorited: boolean) => {
      try {
        if (isFavorited) {
          removeFavorite(trackId);
          await tauri.toggleFavorite(trackId, false);
        } else {
          addFavorite(trackId);
          await tauri.toggleFavorite(trackId, true);
        }
      } catch (err) {
        // Revert optimistic update
        if (isFavorited) {
          addFavorite(trackId);
        } else {
          removeFavorite(trackId);
        }
        console.error("Failed to toggle favorite:", err);
      }
    },
    [addFavorite, removeFavorite],
  );

  const createPlaylist = useCallback(
    async (name: string, description?: string) => {
      try {
        await tauri.createPlaylist(name, description);
        await loadPlaylists();
      } catch (err) {
        console.error("Failed to create playlist:", err);
      }
    },
    [loadPlaylists],
  );

  const addToPlaylist = useCallback(
    async (playlistId: string, trackId: string) => {
      try {
        await tauri.addToPlaylist(playlistId, trackId);
      } catch (err) {
        console.error("Failed to add to playlist:", err);
      }
    },
    [],
  );

  return {
    loadPlaylists,
    loadFavorites,
    loadMoreFavorites,
    toggleFavorite,
    createPlaylist,
    addToPlaylist,
  };
}
