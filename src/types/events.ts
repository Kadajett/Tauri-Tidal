import type { PlaybackState } from "./player";

export interface ProgressPayload {
  position: number;
  duration: number;
  position_fraction: number;
}

export interface TrackChangedPayload {
  track_id: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  artwork_url?: string;
  codec?: string;
  quality?: string;
}

export interface StateChangedPayload {
  state: PlaybackState;
}
