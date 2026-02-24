import { create } from "zustand";
import type { Playlist, Track } from "@/types/track";

interface LibraryStoreState {
  playlists: Playlist[];
  favorites: Track[];
  favoriteTrackIds: Set<string>;
  favoritesNextCursor: string | null;
  favoritesHasMore: boolean;
  loading: boolean;
  loadingMore: boolean;

  setPlaylists: (playlists: Playlist[]) => void;
  setFavorites: (favorites: Track[], nextCursor: string | null, hasMore: boolean) => void;
  appendFavorites: (tracks: Track[], nextCursor: string | null, hasMore: boolean) => void;
  addFavorite: (trackId: string) => void;
  removeFavorite: (trackId: string) => void;
  isFavorite: (trackId: string) => boolean;
  setLoading: (loading: boolean) => void;
  setLoadingMore: (loadingMore: boolean) => void;
}

export const useLibraryStore = create<LibraryStoreState>((set, get) => ({
  playlists: [],
  favorites: [],
  favoriteTrackIds: new Set(),
  favoritesNextCursor: null,
  favoritesHasMore: false,
  loading: false,
  loadingMore: false,

  setPlaylists: (playlists) => set({ playlists }),
  setFavorites: (favorites, nextCursor, hasMore) =>
    set({
      favorites,
      favoriteTrackIds: new Set(favorites.map((t) => t.id)),
      favoritesNextCursor: nextCursor,
      favoritesHasMore: hasMore,
    }),
  appendFavorites: (tracks, nextCursor, hasMore) =>
    set((state) => {
      const combined = [...state.favorites, ...tracks];
      return {
        favorites: combined,
        favoriteTrackIds: new Set(combined.map((t) => t.id)),
        favoritesNextCursor: nextCursor,
        favoritesHasMore: hasMore,
      };
    }),
  addFavorite: (trackId) =>
    set((state) => ({
      favoriteTrackIds: new Set([...state.favoriteTrackIds, trackId]),
    })),
  removeFavorite: (trackId) =>
    set((state) => {
      const next = new Set(state.favoriteTrackIds);
      next.delete(trackId);
      return { favoriteTrackIds: next };
    }),
  isFavorite: (trackId) => get().favoriteTrackIds.has(trackId),
  setLoading: (loading) => set({ loading }),
  setLoadingMore: (loadingMore) => set({ loadingMore }),
}));
