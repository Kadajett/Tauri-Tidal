import { create } from "zustand";
import type { Playlist, Track } from "@/types/track";

interface LibraryStoreState {
  playlists: Playlist[];
  favorites: Track[];
  favoriteTrackIds: Set<string>;
  loading: boolean;

  setPlaylists: (playlists: Playlist[]) => void;
  setFavorites: (favorites: Track[]) => void;
  addFavorite: (trackId: string) => void;
  removeFavorite: (trackId: string) => void;
  isFavorite: (trackId: string) => boolean;
  setLoading: (loading: boolean) => void;
}

export const useLibraryStore = create<LibraryStoreState>((set, get) => ({
  playlists: [],
  favorites: [],
  favoriteTrackIds: new Set(),
  loading: false,

  setPlaylists: (playlists) => set({ playlists }),
  setFavorites: (favorites) =>
    set({
      favorites,
      favoriteTrackIds: new Set(favorites.map((t) => t.id)),
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
}));
