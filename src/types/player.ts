export type PlaybackState = "playing" | "paused" | "stopped" | "buffering";

export type RepeatMode = "off" | "all" | "one";

export interface QueueState {
  tracks: import("./track").Track[];
  currentIndex: number | null;
  repeatMode: RepeatMode;
  shuffled: boolean;
}
