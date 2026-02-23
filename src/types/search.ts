import type { Album, Artist, Playlist, Track } from "./track";

export interface SearchResults {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
}
