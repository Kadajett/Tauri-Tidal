import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AuthStatus } from "@/types/api";
import type { Album, Artist, Playlist, Track } from "@/types/track";
import type { QueueState, RepeatMode } from "@/types/player";
import type { SearchResults } from "@/types/search";
import type {
  ProgressPayload,
  TrackChangedPayload,
  StateChangedPayload,
} from "@/types/events";

// Auth commands
export const checkAuthStatus = () => invoke<AuthStatus>("check_auth_status");
export const login = () => invoke<string>("login");
export const handleAuthCallback = (code: string) =>
  invoke<AuthStatus>("handle_auth_callback", { code });
export const initClientCredentials = () =>
  invoke<void>("init_client_credentials");

// Playback commands
export const playTrack = (trackId: string) =>
  invoke<void>("play_track", { trackId });
export const playTracks = (tracks: Track[], startIndex: number) =>
  invoke<void>("play_tracks", { tracks, startIndex });
export const pausePlayback = () => invoke<void>("pause");
export const resumePlayback = () => invoke<void>("resume");
export const stopPlayback = () => invoke<void>("stop");
export const seekTo = (position: number) =>
  invoke<void>("seek", { position });
export const setVolume = (volume: number) =>
  invoke<void>("set_volume", { volume });
export const getVolume = () => invoke<number>("get_volume");
export const getPlaybackState = () => invoke<string>("get_playback_state");
export const nextTrack = () => invoke<void>("next_track");
export const previousTrack = () => invoke<void>("previous_track");

// Player prefs
export const getPlayerPrefs = () =>
  invoke<{ volume: number; muted: boolean }>("get_player_prefs");
export const savePlayerPrefs = (volume: number, muted: boolean) =>
  invoke<void>("save_player_prefs", { volume, muted });

// Queue commands
export const getQueue = () => invoke<QueueState>("get_queue");
export const addToQueue = (trackId: string) =>
  invoke<void>("add_to_queue", { trackId });
export const removeFromQueue = (index: number) =>
  invoke<void>("remove_from_queue", { index });
export const reorderQueue = (from: number, to: number) =>
  invoke<void>("reorder_queue", { from, to });
export const shuffleQueue = () => invoke<void>("shuffle_queue");
export const unshuffleQueue = () => invoke<void>("unshuffle_queue");
export const toggleRepeat = () => invoke<RepeatMode>("toggle_repeat");
export const clearQueue = () => invoke<void>("clear_queue");
export const playQueueTrack = (index: number) =>
  invoke<void>("play_queue_track", { index });
export const saveQueueState = () => invoke<void>("save_queue_state");
export const loadSavedQueue = () => invoke<QueueState>("load_saved_queue");

// Search commands
export const searchTidal = (query: string, limit?: number) =>
  invoke<SearchResults>("search", { query, limit });
export const searchSuggestions = (query: string) =>
  invoke<string[]>("search_suggestions", { query });

// Playlist commands
export const getPlaylists = () => invoke<Playlist[]>("get_playlists");
export const getPlaylist = (playlistId: string) =>
  invoke<Playlist>("get_playlist", { playlistId });
export const getPlaylistTracks = (playlistId: string) =>
  invoke<Track[]>("get_playlist_tracks", { playlistId });
export const createPlaylist = (name: string, description?: string) =>
  invoke<Playlist>("create_playlist", { name, description });
export const addToPlaylist = (playlistId: string, trackId: string) =>
  invoke<void>("add_to_playlist", { playlistId, trackId });
export const removeFromPlaylist = (playlistId: string, trackId: string) =>
  invoke<void>("remove_from_playlist", { playlistId, trackId });
export const deletePlaylist = (playlistId: string) =>
  invoke<void>("delete_playlist", { playlistId });

// Favorites commands
export const getFavorites = () => invoke<Track[]>("get_favorites");
export const toggleFavorite = (trackId: string, add: boolean) =>
  invoke<void>("toggle_favorite", { trackId, add });

// Browse commands
export const getAlbum = (albumId: string) =>
  invoke<Album>("get_album", { albumId });
export const getAlbumTracks = (albumId: string) =>
  invoke<Track[]>("get_album_tracks", { albumId });
export const getArtist = (artistId: string) =>
  invoke<Artist>("get_artist", { artistId });
export const getArtistAlbums = (artistId: string) =>
  invoke<Album[]>("get_artist_albums", { artistId });
export const getRecommendations = () =>
  invoke<Track[]>("get_recommendations");
export const getSimilarTracks = (trackId: string) =>
  invoke<Track[]>("get_similar_tracks", { trackId });

// Event listeners
export const onProgress = (
  handler: (payload: ProgressPayload) => void
): Promise<UnlistenFn> =>
  listen<ProgressPayload>("playback:progress", (e) => handler(e.payload));

export const onTrackChanged = (
  handler: (payload: TrackChangedPayload) => void
): Promise<UnlistenFn> =>
  listen<TrackChangedPayload>("playback:track-changed", (e) =>
    handler(e.payload)
  );

export const onStateChanged = (
  handler: (payload: StateChangedPayload) => void
): Promise<UnlistenFn> =>
  listen<StateChangedPayload>("playback:state-changed", (e) =>
    handler(e.payload)
  );

export const onTrackEnded = (handler: () => void): Promise<UnlistenFn> =>
  listen("playback:track-ended", () => handler());

export const onQueueChanged = (handler: () => void): Promise<UnlistenFn> =>
  listen("playback:queue-changed", () => handler());
