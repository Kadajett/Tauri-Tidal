export interface Track {
  id: string;
  title: string;
  duration: number;
  trackNumber?: number;
  volumeNumber?: number;
  isrc?: string;
  artistName: string;
  artistId?: string;
  albumName: string;
  albumId?: string;
  artworkUrl?: string;
  mediaTags: string[];
}

export interface FavoritesPage {
  tracks: Track[];
  nextCursor?: string;
  hasMore: boolean;
}

export interface Album {
  id: string;
  title: string;
  artistName: string;
  artistId?: string;
  duration?: number;
  numberOfTracks?: number;
  numberOfVolumes?: number;
  releaseDate?: string;
  artworkUrl?: string;
  mediaTags: string[];
}

export interface Artist {
  id: string;
  name: string;
  pictureUrl?: string;
}

export interface RecommendationSection {
  title: string;
  subtitle?: string;
  tracks: Track[];
}

export interface Playlist {
  id: string;
  name: string;
  description?: string;
  duration?: number;
  numberOfItems?: number;
  playlistType?: string;
  artworkUrl?: string;
  creatorId?: string;
}
